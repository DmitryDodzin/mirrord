use std::{
    collections::{HashMap, HashSet},
    convert::Infallible,
    str::FromStr,
};

use bincode::{error::EncodeError, Decode, Encode};
use reqwest::{header::AUTHORIZATION, Body, Method};
use thiserror::Error;
use tokio::sync::mpsc::{self, error::SendError, Receiver, Sender};
use tokio_stream::StreamExt;
use tracing::{error, trace};

#[derive(Error, Debug)]
pub enum ConnectionError {
    #[error("reqwest error {0}")]
    ReqwestError(#[from] reqwest::Error),
    #[error("decode error {0}")]
    MessageDecodeError(#[from] bincode::error::DecodeError),
    #[error("invalid method {0}")]
    InvalidMethod(#[from] http::method::InvalidMethod),
    #[error("falied to serialize {0}")]
    SerializationError(#[from] EncodeError),
    #[error("request failed to send {0}")]
    ProxiedRequestDropped(#[from] SendError<ProxiedRequest>),
    #[error("response failed to send {0}")]
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
pub struct ProxiedError {
    pub message: String,
}

#[derive(Debug, Encode, Decode)]
pub struct ProxiedResponse {
    pub request_id: u64,
    pub payload: Result<(u16, HttpPayload), ProxiedError>,
}

#[derive(Debug, Encode, Decode)]
pub struct LayerRegisterReply {
    pub user: String,
    pub uid: String,
    pub domain: String,
}

#[derive(Debug)]
pub enum ConnectionStatus {
    Connecting,
    Connected(String),
    Error(ConnectionError),
    Disconnected,
}

#[derive(Default, Clone, Debug)]
pub struct FilterPorts {
    ranges: Vec<(u32, u32)>,
    specific: HashSet<u32>,
}

impl FilterPorts {
    fn is_match(&self, port: u32) -> bool {
        for (start, end) in &self.ranges {
            if port > *start && port < *end {
                return true;
            }
        }

        self.specific.contains(&port)
    }
}

impl FromStr for FilterPorts {
    type Err = <u32 as FromStr>::Err;

    fn from_str(source: &str) -> Result<Self, <Self as FromStr>::Err> {
        let mut filter = Self::default();

        for part in source.split(',') {
            if part.contains("..") {
                let part: Vec<&str> = part.splitn(2, "..").collect();

                filter.ranges.push((part[0].parse()?, part[1].parse()?));
            } else {
                filter.specific.insert(part.parse()?);
            }
        }

        Ok(filter)
    }
}

#[derive(Default, Debug)]
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
        .get(&request_url)
        .header(AUTHORIZATION, auth_header.clone())
        .send()
        .await?
        .bytes()
        .await?;

    let (register, _) = bincode::decode_from_slice::<LayerRegisterReply, _>(
        &register_bytes,
        bincode::config::standard(),
    )?;

    trace!(
        "connect -> url: {:?} | register: {:?}",
        request_url,
        register
    );

    let listen_url = format!("{}/{}/{}", config.server, register.user, register.uid);

    let inbound_connection_status_tx = status_tx.clone();
    let inbound_listen_url = listen_url.clone();
    let inbound_auth_header = auth_header.clone();
    tokio::spawn(async move {
        let _ = inbound_connection_status_tx
            .send(ConnectionStatus::Connecting)
            .await;

        match client
            .get(inbound_listen_url)
            .header(AUTHORIZATION, inbound_auth_header)
            .send()
            .await
            .map(|res| res.bytes_stream())
        {
            Ok(mut stream) => {
                let _ = inbound_connection_status_tx
                    .send(ConnectionStatus::Connected(format!(
                        "{}-{}-<port>.{}",
                        register.user, register.uid, register.domain
                    )))
                    .await;

                while let Some(Ok(bytes)) = stream.next().await {
                    trace!("connect -> inbound -> bytes {:?}(lenght)", bytes.len());

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

                trace!("connect -> inbound -> closed");

                let _ = inbound_connection_status_tx
                    .send(ConnectionStatus::Disconnected)
                    .await;
            }
            Err(err) => {
                let _ = inbound_connection_status_tx
                    .send(ConnectionStatus::Error(ConnectionError::from(err)))
                    .await;
            }
        }
    });

    let outbound_connection_status_tx = status_tx.clone();
    let outbound_listen_url = listen_url;
    let outbound_auth_header = auth_header;
    tokio::spawn(async move {
        let client = reqwest::Client::new();

        let stream = async_stream::stream! {
            while let Some(req) = out_rx.recv().await {
                if let Ok(payload) = bincode::encode_to_vec(req, bincode::config::standard()) {
                    trace!("connect -> outbound -> bytes {:?}(lenght)", payload.len());

                    yield Ok::<_, Infallible>(payload)
                }
            }
        };

        if let Err(err) = client
            .post(&outbound_listen_url)
            .header(AUTHORIZATION, outbound_auth_header)
            .body(Body::wrap_stream(stream))
            .send()
            .await
        {
            let _ = outbound_connection_status_tx
                .send(ConnectionStatus::Error(ConnectionError::from(err)))
                .await;
        }

        trace!("connect -> outbound -> closed");

        let _ = outbound_connection_status_tx
            .send(ConnectionStatus::Disconnected)
            .await;
    });

    tokio::spawn(wrap_connection(out_tx, in_rx, config));

    Ok(status_rx)
}

pub async fn wrap_connection(
    tx: Sender<ProxiedResponse>,
    mut rx: Receiver<ProxiedRequest>,
    config: PreviewConfig,
) {
    trace!("wrap_connection -> config {:?}", config);

    let client = reqwest::Client::new();

    while let Some(req) = rx.recv().await {
        let request_id = req.request_id;

        let response = if config
            .allow_ports
            .as_ref()
            .map(|list| list.is_match(req.port))
            .unwrap_or(true)
            && !config.deny_ports.is_match(req.port)
        {
            let payload = handle_proxied_message(&client, req).await;

            ProxiedResponse {
                request_id,
                payload,
            }
        } else {
            let payload = HttpPayload {
                headers: HashMap::new(),
                body: b"Not Allowed Port".to_vec(),
            };

            ProxiedResponse {
                request_id,
                payload: Ok((401, payload)),
            }
        };

        trace!("wrap_connection -> response {:?}", response);

        let _ = tx
            .send(response)
            .await
            .map_err(|err| error!("wrap_connection -> error {}", err));
    }
}

async fn handle_proxied_message(
    client: &reqwest::Client,
    req: ProxiedRequest,
) -> Result<(u16, HttpPayload), ProxiedError> {
    trace!("handle_proxied_message -> {:?}", req);

    let ProxiedRequest {
        method,
        port,
        path,
        payload,
        ..
    } = req;

    let url = format!("http://127.0.0.1:{}{}", port, path);

    let method = Method::from_bytes(method.as_bytes()).map_err(|err| ProxiedError {
        message: format!("method parse error:\n{}", err),
    })?;

    let mut builder = client.request(method, url);

    for (name, value) in payload.headers {
        builder = builder.header(name, value);
    }

    let response = builder
        .body(payload.body)
        .send()
        .await
        .map_err(|err| ProxiedError {
            message: format!("proxy error:\n{}", err),
        })?;

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

    response
        .bytes()
        .await
        .map(|body| {
            (
                status,
                HttpPayload {
                    headers,
                    body: body.to_vec(),
                },
            )
        })
        .map_err(|err| ProxiedError {
            message: format!("response read error:\n{}", err),
        })
}
