use std::io;

use actix_codec::{Decoder, Encoder};
use bincode::{error::DecodeError, Decode, Encode};
use bytes::{Buf, BufMut, BytesMut};
use mirrord_config::{agent::AgentConfig, target::TargetConfig};
use mirrord_protocol::{ClientMessage, DaemonMessage};

#[derive(Encode, Decode, Debug, PartialEq, Eq, Clone)]
pub struct AgentInitialize {
    #[bincode(with_serde)]
    pub agent: AgentConfig,
    #[bincode(with_serde)]
    pub target: TargetConfig,
}

#[derive(Encode, Decode, Debug, PartialEq, Eq, Clone)]
pub enum OperatorMessage {
    Initialize(AgentInitialize),
    Client(ClientMessage),
    Daemon(DaemonMessage),
}

pub struct OperatorCodec {
    config: bincode::config::Configuration,
}

impl OperatorCodec {
    pub fn new() -> Self {
        OperatorCodec {
            config: bincode::config::standard(),
        }
    }
}

impl Default for OperatorCodec {
    fn default() -> Self {
        OperatorCodec::new()
    }
}

impl Decoder for OperatorCodec {
    type Item = OperatorMessage;
    type Error = io::Error;

    fn decode(&mut self, src: &mut BytesMut) -> io::Result<Option<Self::Item>> {
        match bincode::decode_from_slice(&src[..], self.config) {
            Ok((message, read)) => {
                src.advance(read);
                Ok(Some(message))
            }
            Err(DecodeError::UnexpectedEnd { .. }) => Ok(None),
            Err(err) => Err(io::Error::new(io::ErrorKind::Other, err.to_string())),
        }
    }
}

impl Encoder<OperatorMessage> for OperatorCodec {
    type Error = io::Error;

    fn encode(&mut self, msg: OperatorMessage, dst: &mut BytesMut) -> Result<(), Self::Error> {
        let encoded = bincode::encode_to_vec(msg, self.config)
            .map_err(|err| io::Error::new(io::ErrorKind::Other, err.to_string()))?;
        dst.reserve(encoded.len());
        dst.put(&encoded[..]);

        Ok(())
    }
}
