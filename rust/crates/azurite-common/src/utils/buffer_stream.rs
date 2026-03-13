use std::{
    io,
    pin::Pin,
    task::{Context, Poll},
};

use bytes::Bytes;
use tokio::io::{AsyncRead, ReadBuf};

#[allow(non_snake_case)]
#[derive(Clone, Debug)]
pub struct BufferStream {
    buffer: Bytes,
    offset: usize,
    chunkSize: usize,
    bufferSize: usize,
}

impl BufferStream {
    pub fn new(buffer: Bytes) -> Self {
        let bufferSize = buffer.len();
        Self {
            buffer,
            offset: 0,
            chunkSize: 64 * 1024,
            bufferSize,
        }
    }

    pub fn from_vec(buffer: Vec<u8>) -> Self {
        Self::new(Bytes::from(buffer))
    }

    fn _readNextChunk(&mut self) -> Option<Bytes> {
        if self.offset < self.bufferSize {
            let mut end = self.offset + self.chunkSize;
            if end > self.bufferSize {
                end = self.bufferSize;
            }
            let data = self.buffer.slice(self.offset..end);
            self.offset = end;
            Some(data)
        } else {
            None
        }
    }
}

impl io::Read for BufferStream {
    fn read(&mut self, buf: &mut [u8]) -> io::Result<usize> {
        if buf.is_empty() {
            return Ok(0);
        }
        let Some(chunk) = self._readNextChunk() else {
            return Ok(0);
        };
        let count = buf.len().min(chunk.len());
        buf[..count].copy_from_slice(&chunk[..count]);
        if count < chunk.len() {
            self.offset = self.offset.saturating_sub(chunk.len() - count);
        }
        Ok(count)
    }
}

impl AsyncRead for BufferStream {
    fn poll_read(
        mut self: Pin<&mut Self>,
        _cx: &mut Context<'_>,
        buf: &mut ReadBuf<'_>,
    ) -> Poll<io::Result<()>> {
        if self.offset >= self.bufferSize {
            return Poll::Ready(Ok(()));
        }

        let remaining = self.bufferSize - self.offset;
        let chunkSize = remaining.min(self.chunkSize).min(buf.remaining());
        let end = self.offset + chunkSize;
        buf.put_slice(&self.buffer[self.offset..end]);
        self.offset = end;
        Poll::Ready(Ok(()))
    }
}
