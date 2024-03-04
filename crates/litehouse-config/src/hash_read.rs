use pin_project_lite::pin_project;
use std::task::Poll;
use tokio::io::AsyncRead;

use digest::Digest;

pin_project! {
    pub struct HashRead<T, H> {
        #[pin]
        inner: T,
        hasher: H,
    }
}

impl<T, H: Digest> HashRead<T, H> {
    pub fn new(inner: T, hasher: H) -> Self {
        Self { inner, hasher }
    }

    pub fn finalize(self) -> digest::Output<H> {
        self.hasher.finalize()
    }
}

impl<T: AsyncRead, H: Digest> AsyncRead for HashRead<T, H> {
    fn poll_read(
        self: std::pin::Pin<&mut Self>,
        cx: &mut std::task::Context<'_>,
        buf: &mut tokio::io::ReadBuf<'_>,
    ) -> std::task::Poll<std::io::Result<()>> {
        let this = self.project();
        let before_len = buf.filled().len();

        // Pass on the Poll result, updating the hasher if some new data was written to the buffer.
        match this.inner.poll_read(cx, buf) {
            Poll::Pending => Poll::Pending,
            Poll::Ready(Err(e)) => Poll::Ready(Err(e)),
            Poll::Ready(Ok(())) => {
                let filled = buf.filled();
                let after_len = filled.len();

                if after_len > before_len {
                    // new data was placed in the buffer, update the hasher with newly written data.
                    let new = &filled[before_len..];
                    this.hasher.update(new);
                }

                Poll::Ready(Ok(()))
            }
        }
    }
}
