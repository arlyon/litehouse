use std::task::Poll;

use opendal::raw::oio::{Read, Write};
use opendal::raw::{
    oio, Accessor, Layer, LayeredAccessor, OpList, OpRead, OpWrite, RpList, RpRead, RpWrite,
};
use opendal::Result;

pub struct CacheLayer<A: Accessor>(A);

impl<A: Accessor> CacheLayer<A> {
    pub fn new(a: A) -> Self {
        Self(a)
    }
}

impl<A: Accessor, C: Accessor + Clone> Layer<A> for CacheLayer<C> {
    type LayeredAccessor = CacheLayeredAccessor<A, C>;

    fn layer(&self, inner: A) -> Self::LayeredAccessor {
        CacheLayeredAccessor(inner, self.0.clone())
    }
}

#[derive(Debug)]
pub struct CacheLayeredAccessor<A: Accessor, C: Accessor>(A, C);

#[async_trait::async_trait]
impl<A: Accessor, C: Accessor> LayeredAccessor for CacheLayeredAccessor<A, C> {
    type Inner = A;

    type Reader = Either<TeeReader<A::Reader, C::Writer>, C::Reader>;

    type BlockingReader = A::BlockingReader;

    type Writer = A::Writer;

    type BlockingWriter = A::BlockingWriter;

    type Lister = A::Lister;

    type BlockingLister = A::BlockingLister;

    fn inner(&self) -> &Self::Inner {
        &self.0
    }

    /// Attempt to read from the intermediary store. If that fails, fall back to the inner store,
    /// and write the result to the intermediary store.
    async fn read(&self, path: &str, args: OpRead) -> Result<(RpRead, Self::Reader)> {
        if let Ok((read, reader)) = self.1.read(path, args.clone()).await {
            return Ok((read, Either::B(reader)));
        }

        let (read, reader) = self.0.read(path, args).await?;
        let (_, writer) = self.1.write(path, Default::default()).await.unwrap();

        return Ok((read, Either::A(TeeReader::new(reader, writer))));
    }

    fn blocking_read(&self, _path: &str, _args: OpRead) -> Result<(RpRead, Self::BlockingReader)> {
        todo!()
    }

    async fn write(&self, path: &str, args: OpWrite) -> Result<(RpWrite, Self::Writer)> {
        self.inner().write(path, args).await
    }

    fn blocking_write(&self, path: &str, args: OpWrite) -> Result<(RpWrite, Self::BlockingWriter)> {
        self.inner().blocking_write(path, args)
    }

    async fn list(&self, path: &str, args: OpList) -> Result<(RpList, Self::Lister)> {
        self.inner().list(path, args).await
    }

    fn blocking_list(&self, path: &str, args: OpList) -> Result<(RpList, Self::BlockingLister)> {
        self.inner().blocking_list(path, args)
    }
}

pub enum Either<A, B> {
    A(A),
    B(B),
}

impl<A: oio::Read, B: oio::Read> oio::Read for Either<A, B> {
    fn poll_read(
        &mut self,
        cx: &mut std::task::Context<'_>,
        buf: &mut [u8],
    ) -> std::task::Poll<Result<usize>> {
        match self {
            Either::A(a) => a.poll_read(cx, buf),
            Either::B(b) => b.poll_read(cx, buf),
        }
    }

    fn poll_seek(
        &mut self,
        cx: &mut std::task::Context<'_>,
        pos: std::io::SeekFrom,
    ) -> std::task::Poll<Result<u64>> {
        match self {
            Either::A(a) => a.poll_seek(cx, pos),
            Either::B(b) => b.poll_seek(cx, pos),
        }
    }

    fn poll_next(
        &mut self,
        cx: &mut std::task::Context<'_>,
    ) -> std::task::Poll<Option<Result<bytes::Bytes>>> {
        match self {
            Either::A(a) => a.poll_next(cx),
            Either::B(b) => b.poll_next(cx),
        }
    }
}

/// A reader impl that reads from a source while writing to a sink
///
/// Some runtimes will randomly allocate a buffer for the read, meaning
/// we cannot reuse the buffer for the write. We will need to copy the
/// data into an intermediate buffer before writing to the sink.
pub struct TeeReader<Src, Sink> {
    src: Src,
    sink: Sink,
    outstanding_write: Option<(usize, usize)>,
    buffer: Vec<u8>,
}

impl<Src, Sink> TeeReader<Src, Sink> {
    pub fn new(src: Src, sink: Sink) -> Self {
        Self {
            src,
            sink,
            outstanding_write: None,
            buffer: vec![0; 4096],
        }
    }
}

impl<Src: Read, Sink: Write> Read for TeeReader<Src, Sink> {
    fn poll_read(
        &mut self,
        cx: &mut std::task::Context<'_>,
        buf: &mut [u8],
    ) -> std::task::Poll<Result<usize>> {
        if let Some((idx, total)) = self.outstanding_write {
            match self.sink.poll_write(cx, &&self.buffer[idx..total]) {
                Poll::Ready(Ok(written)) => {
                    let idx_next = idx + written;
                    if idx_next == total {
                        self.outstanding_write = None;
                        buf[0..total].copy_from_slice(&self.buffer[0..total]);
                        Poll::Ready(Ok(total))
                    } else {
                        self.outstanding_write = Some((idx_next, total));
                        Poll::Pending
                    }
                }
                Poll::Ready(Err(e)) => Poll::Ready(Err(e)),
                Poll::Pending => Poll::Pending,
            }
        } else {
            match self.src.poll_read(cx, &mut self.buffer) {
                Poll::Ready(Ok(0)) => Poll::Ready(Ok(0)),
                // we have read some bytes, initiate a write
                Poll::Ready(Ok(n)) => {
                    self.outstanding_write = Some((0, n));
                    self.poll_read(cx, buf)
                }
                Poll::Ready(Err(e)) => Poll::Ready(Err(e)),
                Poll::Pending => Poll::Pending,
            }
        }
    }

    fn poll_seek(
        &mut self,
        cx: &mut std::task::Context<'_>,
        pos: std::io::SeekFrom,
    ) -> std::task::Poll<Result<u64>> {
        println!("seeking source");
        self.src.poll_seek(cx, pos)
    }

    fn poll_next(
        &mut self,
        cx: &mut std::task::Context<'_>,
    ) -> std::task::Poll<Option<Result<bytes::Bytes>>> {
        println!("next");
        self.src.poll_next(cx)
    }
}
