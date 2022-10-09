use std::{collections::HashMap, time::Duration};

use bytes::Bytes;
use mirrord_auth::AuthConfig;
use reqwest::{Body, Method};
use tokio::sync::{
    mpsc::{self, Receiver, Sender},
    RwLock,
};
use tokio_stream::StreamExt;
use tracing::{error, trace};

use crate::{
    connection::{ConnectionError, ConnectionStatus},
    proxy::{
        HttpPayload, LayerRegisterReply, ProxiedError, ProxiedRequest, ProxiedRequestPayload,
        ProxiedResponse,
    },
    PreviewConfig,
};

#[derive(Clone)]
struct ConnectionConfig {
    auth_config: AuthConfig,
    listen_url: String,
    status_tx: Sender<ConnectionStatus>,
}

#[derive(Debug)]
pub enum UpdateMessage {
    PortRemap(u32, u32),
}

pub async fn connect(
    config: PreviewConfig,
    update_rx: Receiver<UpdateMessage>,
) -> Result<Receiver<ConnectionStatus>, ConnectionError> {
    let auth_config = {
        let mut auth_config = AuthConfig::load()?;

        if let Err(_) = auth_config.verify_async(&config.auth_server).await {
            trace!(
                "connect -> refresh_token -> auth_server {}",
                &config.auth_server
            );

            auth_config = auth_config.refresh_async(&config.auth_server).await?;
            auth_config.save()?;
        }

        trace!("connect -> auth_config {:?}", auth_config);

        auth_config
    };

    let (out_tx, out_rx) = mpsc::channel(100);
    let (in_tx, in_rx) = mpsc::channel(100);
    let (status_tx, status_rx) = mpsc::channel(100);

    let request_url = match config.username {
        Some(ref user) => format!("{}/{}", config.server, user),
        None => config.server.clone(),
    };

    let register_bytes = reqwest::Client::new()
        .get(&request_url)
        .bearer_auth(auth_config.access_token.secret())
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
        auth_config,
        status_tx,
        listen_url: format!("{}/{}/{}", config.server, register.user, register.uid),
    };

    tokio::spawn(handle_inbound(connection_config.clone(), register, in_tx));

    tokio::spawn(handle_outbound(connection_config.clone(), out_rx));

    tokio::spawn(wrap_connection(
        out_tx,
        in_rx,
        config,
        connection_config,
        update_rx,
    ));

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
        .bearer_auth(config.auth_config.access_token.secret())
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
    let client = match reqwest::Client::builder()
        .tcp_keepalive(Duration::from_secs(10))
        .http2_keep_alive_interval(Duration::from_secs(1))
        .http2_keep_alive_timeout(Duration::from_secs(10))
        .http2_keep_alive_while_idle(true)
        .build()
    {
        Ok(client) => client,
        Err(err) => {
            let _ = config
                .status_tx
                .send(ConnectionStatus::Error(ConnectionError::from(err)))
                .await;

            let _ = config.status_tx.send(ConnectionStatus::Disconnected).await;

            return;
        }
    };

    while let Some(req) = out_rx.recv().await {
        if let Ok(payload) = bincode::encode_to_vec(req, bincode::config::standard()) {
            trace!("connect -> outbound -> bytes {:?}(lenght)", payload.len());

            let status_tx = config.status_tx.clone();

            let request = client
                .post(&config.listen_url)
                .bearer_auth(config.auth_config.access_token.secret())
                .body(Body::from(payload));

            tokio::spawn(async move {
                if let Err(err) = request.send().await.and_then(|res| res.error_for_status()) {
                    let _ = status_tx
                        .send(ConnectionStatus::Error(ConnectionError::from(err)))
                        .await;
                }
            });
        }
    }
}

async fn wrap_connection(
    tx: Sender<ProxiedResponse>,
    mut rx: Receiver<ProxiedRequest>,
    config: PreviewConfig,
    connection_config: ConnectionConfig,
    mut update_rx: Receiver<UpdateMessage>,
) {
    trace!("wrap_connection -> config {:?}", config);

    let client = match reqwest::Client::builder()
        .tcp_keepalive(Duration::from_secs(10))
        .http2_keep_alive_interval(Duration::from_secs(1))
        .http2_keep_alive_timeout(Duration::from_secs(10))
        .http2_keep_alive_while_idle(true)
        .build()
    {
        Ok(client) => client,
        Err(err) => {
            let _ = connection_config
                .status_tx
                .send(ConnectionStatus::Error(ConnectionError::from(err)))
                .await;

            let _ = connection_config
                .status_tx
                .send(ConnectionStatus::Disconnected)
                .await;

            return;
        }
    };

    let port_remapper = RwLock::new(HashMap::new());

    loop {
        tokio::select! {
            Some(mut req) = rx.recv() => {
                trace!("wrap_connection -> request_id {:?}", req.request_id);

                let request_id = req.request_id;

                let response = if config
                    .allow_ports
                    .as_ref()
                    .map(|list| list.is_match(req.port))
                    .unwrap_or(true)
                    && !config.deny_ports.is_match(req.port)
                {
                    if let Some(remapped_port) = port_remapper.read().await.get(&req.port) {
                        req.port = *remapped_port;
                    }

                    match get_proxied_message(&client, &connection_config, &req).await {
                        Ok(req_payload) => {
                            let payload = handle_proxied_message(&client, req, req_payload).await;

                            ProxiedResponse {
                                request_id,
                                payload,
                            }
                        }
                        Err(err) => {
                            let _ = connection_config
                                .status_tx
                                .send(ConnectionStatus::Error(err))
                                .await;

                            continue;
                        }
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

                trace!("wrap_connection -> response -> request_id {:?}", response.request_id);

                let _ = tx
                    .send(response)
                    .await
                    .map_err(|err| error!("wrap_connection -> error {}", err));
            },
            Some(update) = update_rx.recv(), if config.listen_for_updates => {
                trace!("wrap_connection -> update {:?}", update);

                match update {
                    UpdateMessage::PortRemap(source, target) => {
                        port_remapper.write().await.insert(source, target);
                    }
                }

            }
            else => {
                break;
            }
        }
    }
}

async fn get_proxied_message(
    client: &reqwest::Client,
    config: &ConnectionConfig,
    req: &ProxiedRequest,
) -> Result<HttpPayload, ConnectionError> {
    let bytes = match &req.payload {
        ProxiedRequestPayload::Body(bytes) => Bytes::from(bytes.clone()),
        ProxiedRequestPayload::Defered => {
            client
                .get(format!("{}/{}", config.listen_url, req.request_id))
                .bearer_auth(config.auth_config.access_token.secret())
                .send()
                .await?
                .bytes()
                .await?
        }
    };

    trace!(
        "get_proxied_message -> request_id {:?} | bytes {:?}(lenght)",
        req.request_id,
        bytes.len()
    );

    bincode::decode_from_slice(&bytes, bincode::config::standard())
        .map(|(payload, _)| payload)
        .map_err(|err| err.into())
}

async fn handle_proxied_message(
    client: &reqwest::Client,
    req: ProxiedRequest,
    payload: HttpPayload,
) -> Result<(u16, HttpPayload), ProxiedError> {
    let ProxiedRequest {
        request_id,
        method,
        port,
        path,
        ..
    } = req;

    trace!(
        "handle_proxied_message -> method {:?} | port {:?} | path {:?} | request_id {:?}",
        method,
        port,
        path,
        request_id
    );

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
