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

#[derive(Encode, Decode)]
pub struct HttpPayload {
    pub headers: HashMap<String, String>,
    pub path: String,
    pub method: String,
    pub body: Vec<u8>,
}

#[derive(Encode, Decode)]
pub struct ProxiedRequest {
    pub port: u32,
    pub payload: HttpPayload,
}

#[derive(Encode, Decode)]
pub struct ProxiedResponse {
    pub payload: HttpPayload,
}

pub async fn connect(
    server: String,
    user: String,
    uid: String,
) -> Result<(Sender<ProxiedRequest>, Receiver<ProxiedResponse>), ConnectionError> {
    let (out_tx, mut out_rx) = mpsc::channel(30);
    let (in_tx, in_rx) = mpsc::channel(30);

    let url = format!("{}/{}/{}", server, user, uid);

    let mut stream = reqwest::get(&url).await?.bytes_stream();

    tokio::spawn(async move {
        while let Some(value) = stream.next().await {
            println!("Got {:?}", value);

            // if let Err(_) = in_tx.send(ProxiedResponse).await {
            //     println!("send dropped");
            // }
        }
    });

    tokio::spawn(async move {
        let client = reqwest::Client::new();

        while let Some(_req) = out_rx.recv().await {
            if let Err(err) = client.post(&url).body("foobar").send().await {
                println!("{:?}", err);
            }
        }
    });

    Ok((out_tx, in_rx))
}
