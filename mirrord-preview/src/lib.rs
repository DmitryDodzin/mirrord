use std::collections::HashMap;

use bincode::{Decode, Encode};
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
    pub method: String,
    pub port: u32,
    pub path: String,
    pub payload: HttpPayload,
}

#[derive(Debug, Encode, Decode)]
pub struct ProxiedResponse {
    pub status: u16,
    pub payload: HttpPayload,
}

pub async fn connect(
    server: String,
    user: String,
    uid: String,
) -> Result<(Sender<ProxiedResponse>, Receiver<ProxiedRequest>), ConnectionError> {
    let (out_tx, mut out_rx) = mpsc::channel(30);
    let (in_tx, in_rx) = mpsc::channel(30);

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

        while let Some(req) = out_rx.recv().await {
            if let Ok(payload) = bincode::encode_to_vec(req, bincode::config::standard()) {
                if let Err(err) = client.post(&url).body(payload).send().await {
                    println!("{:?}", err);
                }
            }
        }
    });

    Ok((out_tx, in_rx))
}
