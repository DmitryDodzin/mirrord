use std::{
    collections::{HashMap, HashSet},
    convert::Infallible,
};

use bincode::{error::EncodeError, Decode, Encode};
use reqwest::{header::AUTHORIZATION, Body, Method};
use thiserror::Error;
use tokio::sync::mpsc::{self, error::SendError, Receiver, Sender};
use tokio_stream::StreamExt;

#[derive(Error, Debug)]
pub enum ConnectionError {
    #[error("reqwest error {0:#?}")]
    ReqwestError(#[from] reqwest::Error),
    #[error("decode error {0:#?}")]
    MessageDecodeError(#[from] bincode::error::DecodeError),
    #[error("invalid method {0:#?}")]
    InvalidMethod(#[from] http::method::InvalidMethod),
    #[error("falied to serialize {0:#?}")]
    SerializationError(#[from] EncodeError),
    #[error("request failed to send {0:#?}")]
    ProxiedRequestDropped(#[from] SendError<ProxiedRequest>),
    #[error("response failed to send {0:#?}")]
    ProxiedResponseDropped(#[from] SendError<ProxiedResponse>),
}

#[derive(Debug, Encode, Decode)]
pub struct HttpPayload {
    pub headers: HashMap<String, String>,
    pub body: Vec<u8>,
}

#[derive(Debug, Encode, Decode)]
pub struct ProxiedRequest {
    pub request_id: u64,
    pub method: String,
    pub port: u32,
    pub path: String,
    pub payload: HttpPayload,
}

#[derive(Debug, Encode, Decode)]
pub struct ProxiedResponse {
    pub request_id: u64,
    pub status: u16,
    pub payload: HttpPayload,
}

#[derive(Debug, Encode, Decode)]
pub struct LayerRegisterReply {
    pub user: String,
    pub uid: String,
}

#[derive(Debug)]
pub enum ConnectionStatus {
    Connecting,
    Connected(String),
    Error(ConnectionError),
}

#[derive(Default)]
pub struct FilterPorts(HashSet<u32>);

impl FilterPorts {
    fn is_allowed(&self, port: u32) -> bool {
        !self.0.contains(&port)
    }
}

#[derive(Default)]
pub struct PreviewConfig {
    pub server: String,
    pub username: Option<String>,
    pub allow_ports: Option<FilterPorts>,
    pub deny_ports: FilterPorts,
}

pub async fn connect(
    token: String,
    config: PreviewConfig,
) -> Result<Receiver<ConnectionStatus>, ConnectionError> {
    let (out_tx, mut out_rx) = mpsc::channel(100);
    let (in_tx, in_rx) = mpsc::channel(100);
    let (status_tx, status_rx) = mpsc::channel(100);

    let request_url = match config.username {
        Some(ref user) => format!("{}/{}", config.server, user),
        None => config.server.clone(),
    };

    let auth_header = format!("Bearer {}", token);

    let client = reqwest::Client::new();

    let register_bytes = client
        .get(request_url)
        .header(AUTHORIZATION, auth_header.clone())
        .send()
        .await?
        .bytes()
        .await?;

    let _ = status_tx.send(ConnectionStatus::Connecting).await;

    let (register, _) = bincode::decode_from_slice::<LayerRegisterReply, _>(
        &register_bytes,
        bincode::config::standard(),
    )?;

    let listen_url = format!("{}/{}/{}", config.server, register.user, register.uid);

    let mut stream = client
        .get(&listen_url)
        .header(AUTHORIZATION, auth_header.clone())
        .send()
        .await?
        .bytes_stream();

    let _ = status_tx
        .send(ConnectionStatus::Connected(format!(
            "{}-{}-<port>.preview.metalbear.co",
            register.user, register.uid
        )))
        .await;

    let inbound_connection_status_tx = status_tx.clone();
    tokio::spawn(async move {
        while let Some(Ok(bytes)) = stream.next().await {
            match bincode::decode_from_slice::<ProxiedRequest, _>(
                &bytes,
                bincode::config::standard(),
            ) {
                Ok((request, _size)) => {
                    if let Err(request) = in_tx.send(request).await {
                        let _ = inbound_connection_status_tx
                            .send(ConnectionStatus::Error(
                                ConnectionError::ProxiedRequestDropped(request),
                            ))
                            .await;
                    }
                }
                Err(err) => {
                    let _ = inbound_connection_status_tx
                        .send(ConnectionStatus::Error(ConnectionError::from(err)))
                        .await;
                }
            }
        }
    });

    let outbound_connection_status_tx = status_tx.clone();
    tokio::spawn(async move {
        let client = reqwest::Client::new();

        let stream = async_stream::stream! {
            while let Some(req) = out_rx.recv().await {
                if let Ok(payload) = bincode::encode_to_vec(req, bincode::config::standard()) {
                    yield Ok::<_, Infallible>(payload)
                }
            }
        };

        if let Err(err) = client
            .post(&listen_url)
            .header(AUTHORIZATION, auth_header)
            .body(Body::wrap_stream(stream))
            .send()
            .await
        {
            let _ = outbound_connection_status_tx
                .send(ConnectionStatus::Error(ConnectionError::from(err)))
                .await;
        }
    });

    let client_connection_status_tx = status_tx.clone();
    tokio::spawn(async move {
        if let Err(err) = wrap_connection(out_tx, in_rx, config).await {
            let _ = client_connection_status_tx
                .send(ConnectionStatus::Error(err))
                .await;
        }
    });

    Ok(status_rx)
}

pub async fn wrap_connection(
    tx: Sender<ProxiedResponse>,
    mut rx: Receiver<ProxiedRequest>,
    config: PreviewConfig,
) -> Result<(), ConnectionError> {
    let client = reqwest::Client::new();

    while let Some(ProxiedRequest {
        method,
        request_id,
        port,
        path,
        payload,
    }) = rx.recv().await
    {
        if config
            .allow_ports
            .as_ref()
            .map(|list| list.is_allowed(port))
            .unwrap_or(false)
            && config.deny_ports.is_allowed(port)
        {
            let url = format!("http://127.0.0.1:{}{}", port, path);

            let method = Method::from_bytes(method.as_bytes())?;

            let mut builder = client.request(method, url);

            for (name, value) in payload.headers {
                builder = builder.header(name, value);
            }

            if let Ok(response) = builder.body(payload.body).send().await {
                let headers = response
                    .headers()
                    .iter()
                    .filter_map(|(name, value)| {
                        value
                            .to_str()
                            .ok()
                            .map(|value| (name.as_str().to_owned(), value.to_owned()))
                    })
                    .collect();

                let status = response.status().as_u16();

                let body = response.bytes().await?.to_vec();

                let payload = HttpPayload { headers, body };

                tx.send(ProxiedResponse {
                    payload,
                    request_id,
                    status,
                })
                .await?;
            }
        } else {
            let payload = HttpPayload {
                headers: HashMap::new(),
                body: b"Not Allowed Port".to_vec(),
            };

            tx.send(ProxiedResponse {
                payload,
                request_id,
                status: 401,
            })
            .await?;
        }
    }

    Ok(())
}
