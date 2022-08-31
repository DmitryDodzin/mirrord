use bincode::error::{DecodeError, EncodeError};
use http::method::InvalidMethod;
use thiserror::Error;
use tokio::sync::mpsc::error::SendError;

use crate::proxy::{ProxiedRequest, ProxiedResponse};

#[derive(Error, Debug)]
pub enum ConnectionError {
    #[error("reqwest error {0}")]
    ReqwestError(#[from] reqwest::Error),
    #[error("decode error {0}")]
    MessageDecodeError(#[from] DecodeError),
    #[error("invalid method {0}")]
    InvalidMethod(#[from] InvalidMethod),
    #[error("falied to serialize {0}")]
    SerializationError(#[from] EncodeError),
    #[error("request failed to send {0}")]
    ProxiedRequestDropped(#[from] SendError<ProxiedRequest>),
    #[error("response failed to send {0}")]
    ProxiedResponseDropped(#[from] SendError<ProxiedResponse>),
}

#[derive(Debug)]
pub enum ConnectionStatus {
    Connecting,
    Connected(String),
    Error(ConnectionError),
    Disconnected,
}
