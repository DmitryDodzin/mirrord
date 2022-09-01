use std::{collections::HashMap, convert::Infallible};

use mirrord_auth::AuthConfig;
use reqwest::{header::AUTHORIZATION, Body, Method};
use tokio::sync::mpsc::{self, Receiver, Sender};
use tokio_stream::StreamExt;
use tracing::{error, trace};

use crate::{
    connection::{ConnectionError, ConnectionStatus},
    proxy::{HttpPayload, LayerRegisterReply, ProxiedError, ProxiedRequest, ProxiedResponse},
    PreviewConfig,
};

#[derive(Clone)]
struct ConnectionConfig {
    auth_header: String,
    listen_url: String,
    status_tx: Sender<ConnectionStatus>,
}

pub async fn connect(config: PreviewConfig) -> Result<Receiver<ConnectionStatus>, ConnectionError> {
    let auth_config = AuthConfig::load()?;

    let (out_tx, out_rx) = mpsc::channel(100);
    let (in_tx, in_rx) = mpsc::channel(100);
    let (status_tx, status_rx) = mpsc::channel(100);

    let request_url = match config.username {
        Some(ref user) => format!("{}/{}", config.server, user),
        None => config.server.clone(),
    };

    let auth_header = format!("Bearer {}", auth_config.access_token);

    let register_bytes = reqwest::Client::new()
        .get(&request_url)
        .header(AUTHORIZATION, &auth_header)
        .send()
        .await?
        .error_for_status()?
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

    let connection_config = ConnectionConfig {
        auth_header,
        status_tx,
        listen_url: format!("{}/{}/{}", config.server, register.user, register.uid),
    };

    tokio::spawn(handle_inbound(connection_config.clone(), register, in_tx));

    tokio::spawn(handle_outbound(connection_config, out_rx));

    tokio::spawn(wrap_connection(out_tx, in_rx, config));

    Ok(status_rx)
}

async fn handle_inbound(
    config: ConnectionConfig,
    register: LayerRegisterReply,
    in_tx: Sender<ProxiedRequest>,
) {
    let client = reqwest::Client::new();

    let _ = config.status_tx.send(ConnectionStatus::Connecting).await;

    match client
        .get(config.listen_url)
        .header(AUTHORIZATION, config.auth_header)
        .send()
        .await
        .and_then(|res| res.error_for_status())
        .map(|res| res.bytes_stream())
    {
        Ok(mut stream) => {
            let _ = config
                .status_tx
                .send(ConnectionStatus::Connected(format!(
                    "https://{}-{}-<port>.{}",
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
                            let _ = config
                                .status_tx
                                .send(ConnectionStatus::Error(
                                    ConnectionError::ProxiedRequestDropped(request),
                                ))
                                .await;
                        }
                    }
                    Err(err) => {
                        let _ = config
                            .status_tx
                            .send(ConnectionStatus::Error(ConnectionError::from(err)))
                            .await;
                    }
                }
            }

            trace!("connect -> inbound -> closed");

            let _ = config.status_tx.send(ConnectionStatus::Disconnected).await;
        }
        Err(err) => {
            let _ = config
                .status_tx
                .send(ConnectionStatus::Error(ConnectionError::from(err)))
                .await;
        }
    }
}

async fn handle_outbound(config: ConnectionConfig, mut out_rx: Receiver<ProxiedResponse>) {
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
        .post(&config.listen_url)
        .header(AUTHORIZATION, config.auth_header)
        .body(Body::wrap_stream(stream))
        .send()
        .await
        .and_then(|res| res.error_for_status())
    {
        let _ = config
            .status_tx
            .send(ConnectionStatus::Error(ConnectionError::from(err)))
            .await;
    }

    trace!("connect -> outbound -> closed");

    let _ = config.status_tx.send(ConnectionStatus::Disconnected).await;
}

async fn wrap_connection(
    tx: Sender<ProxiedResponse>,
    mut rx: Receiver<ProxiedRequest>,
    config: PreviewConfig,
) {
    trace!("wrap_connection -> config {:?}", config);

    let client = reqwest::Client::new();

    while let Some(mut req) = rx.recv().await {
        let request_id = req.request_id;

        let response = if config
            .allow_ports
            .as_ref()
            .map(|list| list.is_match(req.port))
            .unwrap_or(true)
            && !config.deny_ports.is_match(req.port)
        {
            if let Ok(remapper) = config.port_remapper.read() {
                if let Some(remapped_port) = remapper.get(&req.port) {
                    req.port = *remapped_port;
                }
            }

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
