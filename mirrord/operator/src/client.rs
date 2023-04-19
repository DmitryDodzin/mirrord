use futures::{SinkExt, StreamExt};
use http::request::Request;
use kube::{error::ErrorResponse, Api, Client};
use mirrord_config::{target::TargetConfig, LayerConfig};
use mirrord_kube::{
    api::{get_k8s_resource_api, kubernetes::create_kube_api},
    error::KubeApiError,
};
use mirrord_protocol::{ClientMessage, DaemonMessage};
use thiserror::Error;
use tokio::sync::mpsc::{self, Receiver, Sender};
use tokio_tungstenite::tungstenite::{Error as TungsteniteError, Message};
use tracing::{error, warn};

use crate::crd::TargetCrd;

static CONNECTION_CHANNEL_SIZE: usize = 1000;

#[derive(Debug, Error)]
pub enum OperatorApiError {
    #[error("unable to create target for TargetConfig")]
    InvalidTarget,
    #[error(transparent)]
    HttpError(#[from] http::Error),
    #[error(transparent)]
    WsError(#[from] TungsteniteError),
    #[error(transparent)]
    KubeApiError(#[from] KubeApiError),
    #[error(transparent)]
    DecodeError(#[from] bincode::error::DecodeError),
    #[error(transparent)]
    EncodeError(#[from] bincode::error::EncodeError),
    #[error("invalid message: {0:?}")]
    InvalidMessage(Message),
    #[error("Receiver<DaemonMessage> was dropped")]
    DaemonReceiverDropped,
}

type Result<T, E = OperatorApiError> = std::result::Result<T, E>;

pub struct OperatorApi {
    session_id: u64,
    client: Client,
    target_api: Api<TargetCrd>,
    target_config: TargetConfig,
}

impl OperatorApi {
    pub async fn discover(
        config: &LayerConfig,
    ) -> Result<Option<(mpsc::Sender<ClientMessage>, mpsc::Receiver<DaemonMessage>)>> {
        let operator_api = OperatorApi::new(config).await?;

        if let Some(target) = operator_api.fetch_target().await? {
            operator_api.connect_target(target).await.map(Some)
        } else {
            Ok(None)
        }
    }

    async fn new(config: &LayerConfig) -> Result<Self> {
        let target_config = config.target.clone();

        let client = create_kube_api(
            config.accept_invalid_certificates,
            config.kubeconfig.clone(),
        )
        .await?;

        let target_api: Api<TargetCrd> =
            get_k8s_resource_api(&client, target_config.namespace.as_deref());

        Ok(OperatorApi {
            session_id: rand::random(),
            client,
            target_api,
            target_config,
        })
    }

    async fn fetch_target(&self) -> Result<Option<TargetCrd>> {
        let target = self
            .target_config
            .path
            .as_ref()
            .map(TargetCrd::target_name)
            .ok_or(OperatorApiError::InvalidTarget)?;

        match self.target_api.get(&target).await {
            Ok(target) => Ok(Some(target)),
            Err(kube::Error::Api(ErrorResponse { code: 404, .. })) => Ok(None),
            Err(err) => Err(OperatorApiError::from(KubeApiError::from(err))),
        }
    }

    async fn connect_target(
        &self,
        target: TargetCrd,
    ) -> Result<(mpsc::Sender<ClientMessage>, mpsc::Receiver<DaemonMessage>)> {
        let session_id = self.session_id;
        let client = self.client.clone();
        let target_path = format!(
            "{}/{}?connect=true",
            self.target_api.resource_url(),
            target.name()
        );

        let creator = move || {
            let client = client.clone();
            let request = Request::builder()
                .uri(&target_path)
                .header("x-session-id", session_id)
                .body(vec![]);

            async move {
                client
                    .connect(request?)
                    .await
                    .map_err(KubeApiError::from)
                    .map_err(OperatorApiError::from)
            }
        };

        let (connection, mut wrapper) = ConnectionWrapper::wrap(creator().await?);

        tokio::spawn(async move {
            while let Err(err) = wrapper.start().await {
                if let OperatorApiError::WsError(TungsteniteError::Io(io_err)) = err {
                    warn!("Operator communication IOError {io_err}");
                    warn!("Reconnecting...");

                    match creator().await {
                        Ok(new_connection) => wrapper.replace_stream(new_connection),
                        Err(err) => {
                            error!("Error reconnecting to operator {err}")
                        }
                    }
                } else {
                    error!("Error connecting to operator {err}");
                }
            }
        });

        Ok(connection)
    }
}

pub struct ConnectionWrapper<T> {
    connection: T,
    client_rx: Receiver<ClientMessage>,
    daemon_tx: Sender<DaemonMessage>,
}

impl<T> ConnectionWrapper<T>
where
    for<'stream> T: StreamExt<Item = Result<Message, TungsteniteError>>
        + SinkExt<Message, Error = TungsteniteError>
        + Send
        + Unpin
        + 'stream,
{
    fn wrap(
        connection: T,
    ) -> (
        (Sender<ClientMessage>, Receiver<DaemonMessage>),
        ConnectionWrapper<T>,
    ) {
        let (client_tx, client_rx) = mpsc::channel(CONNECTION_CHANNEL_SIZE);
        let (daemon_tx, daemon_rx) = mpsc::channel(CONNECTION_CHANNEL_SIZE);

        let connection_wrapper = ConnectionWrapper {
            connection,
            client_rx,
            daemon_tx,
        };

        ((client_tx, daemon_rx), connection_wrapper)
    }

    fn replace_stream(&mut self, connection: T) {
        self.connection = connection;
    }

    async fn handle_client_message(&mut self, client_message: ClientMessage) -> Result<()> {
        let payload = bincode::encode_to_vec(client_message, bincode::config::standard())?;

        self.connection.send(payload.into()).await?;

        Ok(())
    }

    async fn handle_daemon_message(
        &mut self,
        daemon_message: Result<Message, TungsteniteError>,
    ) -> Result<()> {
        match daemon_message? {
            Message::Binary(payload) => {
                let (daemon_message, _) = bincode::decode_from_slice::<DaemonMessage, _>(
                    &payload,
                    bincode::config::standard(),
                )?;

                self.daemon_tx
                    .send(daemon_message)
                    .await
                    .map_err(|_| OperatorApiError::DaemonReceiverDropped)
            }
            message => Err(OperatorApiError::InvalidMessage(message)),
        }
    }

    async fn start(&mut self) -> Result<()> {
        loop {
            tokio::select! {
                client_message = self.client_rx.recv() => {
                    match client_message {
                        Some(client_message) => self.handle_client_message(client_message).await?,
                        None => break,
                    }
                }
                daemon_message = self.connection.next() => {
                    match daemon_message {
                        Some(daemon_message) => self.handle_daemon_message(daemon_message).await?,
                        None => break,
                    }
                }
            }
        }

        let _ = self.connection.send(Message::Close(None)).await;

        Ok(())
    }
}
