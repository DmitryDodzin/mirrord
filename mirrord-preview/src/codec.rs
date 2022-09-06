use std::{io, marker::PhantomData};

use actix_codec::{Decoder, Encoder};
use bincode::{error::DecodeError, Decode, Encode};
use bytes::{Buf, BufMut, BytesMut};

pub struct BincodeCodec<E, D = E> {
    config: bincode::config::Configuration,
    _encode: PhantomData<E>,
    _decode: PhantomData<D>,
}

impl<E, D> Default for BincodeCodec<E, D> {
    fn default() -> Self {
        BincodeCodec {
            config: bincode::config::standard(),
            _encode: PhantomData::<E>,
            _decode: PhantomData::<D>,
        }
    }
}

impl<E, D> Decoder for BincodeCodec<E, D>
where
    D: Decode,
{
    type Item = D;
    type Error = io::Error;

    fn decode(&mut self, src: &mut BytesMut) -> Result<Option<Self::Item>, Self::Error> {
        match bincode::decode_from_slice(&src[..], self.config) {
            Ok((message, read)) => {
                src.advance(read);
                Ok(Some(message))
            }
            Err(DecodeError::UnexpectedEnd) => Ok(None),
            Err(err) => Err(io::Error::new(io::ErrorKind::Other, err.to_string())),
        }
    }
}

impl<E, D> Encoder<E> for BincodeCodec<E, D>
where
    E: Encode,
{
    type Error = io::Error;

    fn encode(&mut self, payload: E, dst: &mut BytesMut) -> Result<(), Self::Error> {
        let encoded = match bincode::encode_to_vec(payload, self.config) {
            Ok(encoded) => encoded,
            Err(err) => {
                return Err(io::Error::new(io::ErrorKind::Other, err.to_string()));
            }
        };

        dst.reserve(encoded.len());
        dst.put(&encoded[..]);

        Ok(())
    }
}
