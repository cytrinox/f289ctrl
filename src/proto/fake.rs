use std::pin::Pin;

pub(crate) struct FakeBuffer {
    response_buf: Vec<u8>,
}

impl FakeBuffer {
    pub(crate) fn new(response_buf: Vec<u8>) -> Self {
        Self { response_buf }
    }
}

impl tokio::io::AsyncRead for FakeBuffer {
    fn poll_read(
        mut self: Pin<&mut Self>,
        _cx: &mut std::task::Context<'_>,
        buf: &mut tokio::io::ReadBuf<'_>,
    ) -> std::task::Poll<std::io::Result<()>> {
        if !self.response_buf.is_empty() {
            let c = if buf.capacity() < self.response_buf.len() {
                buf.capacity()
            } else {
                self.response_buf.len()
            };
            buf.put_slice(&self.response_buf[0..c]);
            self.response_buf.drain(0..c);
        }
        std::task::Poll::Ready(Ok(()))
    }
}

impl tokio::io::AsyncWrite for FakeBuffer {
    fn poll_write(
        self: Pin<&mut Self>,
        _cx: &mut std::task::Context<'_>,
        buf: &[u8],
    ) -> std::task::Poll<std::result::Result<usize, std::io::Error>> {
        std::task::Poll::Ready(Ok(buf.len()))
    }

    fn poll_flush(
        self: Pin<&mut Self>,
        _cx: &mut std::task::Context<'_>,
    ) -> std::task::Poll<std::result::Result<(), std::io::Error>> {
        std::task::Poll::Ready(Ok(()))
    }

    fn poll_shutdown(
        self: Pin<&mut Self>,
        _cx: &mut std::task::Context<'_>,
    ) -> std::task::Poll<std::result::Result<(), std::io::Error>> {
        std::task::Poll::Ready(Ok(()))
    }
}
