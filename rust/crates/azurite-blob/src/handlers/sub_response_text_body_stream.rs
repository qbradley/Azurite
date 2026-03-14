use crate::generated::i_response::GeneratedWritableStream;

#[derive(Debug, Clone, Default)]
pub struct SubResponseTextBodyStream {
    inner: GeneratedWritableStream,
}

impl SubResponseTextBodyStream {
    pub fn new() -> Self {
        Self::default()
    }

    pub fn stream(&self) -> GeneratedWritableStream {
        self.inner.clone()
    }

    pub fn end(&self) {
        self.inner.end();
    }

    pub fn getBodyContent(&self) -> String {
        self.inner.text()
    }
}
