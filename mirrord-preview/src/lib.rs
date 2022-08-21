use std::{collections::HashMap, convert::Infallible};

use bincode::{Decode, Encode};
use reqwest::Body;
use thiserror::Error;
use tokio::sync::mpsc::{self, Receiver, Sender};
use tokio_stream::StreamExt;

#[derive(Error, Debug)]
pub enum ConnectionError {
    #[error("reqwest error {0:#?}")]
    ReqwestError(#[from] reqwest::Error),
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

pub async fn connect(
    server: String,
    user: String,
    uid: String,
) -> Result<(Sender<ProxiedResponse>, Receiver<ProxiedRequest>), ConnectionError> {
    let (out_tx, mut out_rx) = mpsc::channel(100);
    let (in_tx, in_rx) = mpsc::channel(100);

    let url = format!("{}/{}/{}", server, user, uid);

    let mut stream = reqwest::get(&url).await?.bytes_stream();

    tokio::spawn(async move {
        while let Some(Ok(bytes)) = stream.next().await {
            if let Ok((response, _size)) =
                bincode::decode_from_slice::<ProxiedRequest, _>(&bytes, bincode::config::standard())
            {
                if let Err(_) = in_tx.send(response).await {
                    println!("send dropped");
                }
            }
        }
    });

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
            .post(&url)
            .body(Body::wrap_stream(stream))
            .send()
            .await
        {
            println!("{:#?}", err);
        }
    });

    Ok((out_tx, in_rx))
}
