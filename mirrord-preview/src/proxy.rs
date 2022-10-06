use std::collections::HashMap;

use bincode::{Decode, Encode};

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
