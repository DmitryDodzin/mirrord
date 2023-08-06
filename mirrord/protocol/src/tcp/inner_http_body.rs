use std::{
    collections::VecDeque,
    convert::Infallible,
    fmt,
    pin::Pin,
    task::{Context, Poll},
};

use bytes::{Bytes, BytesMut};
use http_body_util::BodyExt;
use hyper::{
    body::{Body, Frame},
    http::HeaderMap,
};
use serde::{Deserialize, Serialize};

#[derive(Debug, Default, PartialEq, Eq, Clone)]
pub struct InternalHttpBody(Bytes, VecDeque<InternalHttpBodyFrame>);

impl InternalHttpBody {
    pub fn new<B: Into<Bytes>>(bytes: B, frames: VecDeque<InternalHttpBodyFrame>) -> Self {
        InternalHttpBody(bytes.into(), frames)
    }

    pub fn unpack(self) -> (Bytes, VecDeque<InternalHttpBodyFrame>) {
        (self.0, self.1)
    }

    pub fn from_bytes(bytes: Vec<u8>) -> Self {
        let bytes = Bytes::from(bytes);
        let frames = VecDeque::from([InternalHttpBodyFrame::Data(0, bytes.len())]);

        InternalHttpBody(bytes, frames)
    }

    pub async fn from_body<B>(mut body: B) -> Result<Self, B::Error>
    where
        B: Body<Data = Bytes> + Unpin,
    {
        let mut bytes = BytesMut::new();
        let mut frames = VecDeque::new();

        while let Some(frame) = body.frame().await {
            match frame?.into_data() {
                Ok(data) => {
                    let start = bytes.len();
                    bytes.extend(data);
                    let end = bytes.len();
                    frames.push_back(InternalHttpBodyFrame::Data(start, end))
                }
                Err(frame) => {
                    if let Ok(trailers) = frame.into_trailers() {
                        frames.push_back(InternalHttpBodyFrame::Trailers(trailers))
                    }
                }
            }
        }

        Ok(InternalHttpBody(bytes.into(), frames))
    }

    fn next_frame(&mut self) -> Option<Frame<Bytes>> {
        let next_frame = self.1.pop_front()?;

        match next_frame {
            InternalHttpBodyFrame::Data(start, end) => Some(Frame::data(self.0.slice(start..end))),
            InternalHttpBodyFrame::Trailers(map) => Some(Frame::trailers(map)),
        }
    }
}

impl Body for InternalHttpBody {
    type Data = Bytes;

    type Error = Infallible;

    fn poll_frame(
        mut self: Pin<&mut Self>,
        _: &mut Context<'_>,
    ) -> Poll<Option<Result<Frame<Self::Data>, Self::Error>>> {
        Poll::Ready(self.next_frame().map(Ok))
    }

    fn is_end_stream(&self) -> bool {
        self.1.is_empty()
    }
}

#[derive(Serialize, Deserialize, PartialEq, Eq, Clone)]
pub enum InternalHttpBodyFrame {
    Data(usize, usize),
    Trailers(#[serde(with = "http_serde::header_map")] HeaderMap),
}

impl fmt::Debug for InternalHttpBodyFrame {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            InternalHttpBodyFrame::Data(start, end) => f
                .debug_tuple("Data")
                .field(&format_args!("{start}..{end}"))
                .finish(),
            InternalHttpBodyFrame::Trailers(map) => {
                f.debug_tuple("Trailers").field(&map.len()).finish()
            }
        }
    }
}
