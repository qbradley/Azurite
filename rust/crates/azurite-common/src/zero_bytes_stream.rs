use std::{
    cmp::min,
    pin::Pin,
    task::{Context, Poll},
};

use tokio::io::{AsyncRead, ReadBuf};

const zeroBytesRangeUnit: usize = 512;
const zeroBytesChunk: [u8; zeroBytesRangeUnit] = [0; zeroBytesRangeUnit];

#[allow(non_snake_case)]
#[derive(Debug, Clone)]
pub struct ZeroBytesStream {
    pub length: u64,
    leftBytes: u64,
}

impl ZeroBytesStream {
    pub fn new(length: u64) -> Self {
        Self {
            length,
            leftBytes: length,
        }
    }
}

impl AsyncRead for ZeroBytesStream {
    fn poll_read(
        mut self: Pin<&mut Self>,
        _cx: &mut Context<'_>,
        buf: &mut ReadBuf<'_>,
    ) -> Poll<std::io::Result<()>> {
        if self.leftBytes == 0 {
            return Poll::Ready(Ok(()));
        }

        let count = min(
            min(self.leftBytes as usize, buf.remaining()),
            zeroBytesRangeUnit,
        );
        buf.put_slice(&zeroBytesChunk[..count]);
        self.leftBytes -= count as u64;
        Poll::Ready(Ok(()))
    }
}
