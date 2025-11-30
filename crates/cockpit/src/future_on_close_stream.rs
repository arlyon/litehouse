use std::{future::Future, mem::ManuallyDrop, pin::Pin};

use futures::{
    Stream,
    future::{Fuse, FusedFuture},
};

pin_project_lite::pin_project! {
    /// Stream adaptor to run a future once after the stream is closed
    pub struct FutureOnCloseStream<S, F: 'static> where F: Future<Output = ()>, F: Send {
        #[pin]
        stream: S,
        #[pin]
        // we fuse this to allow us to check if the future is terminated
        // otherwise spawning the task causes panics
        future: ManuallyDrop<Fuse<F>>,
    }


    impl<S, F: 'static> PinnedDrop for FutureOnCloseStream<S, F>
    where
        F: Future<Output = ()>,
        F: Send,
    {
        fn drop(mut this: Pin<&mut Self>) {

            let this = this.as_mut().project();
            let future = unsafe { ManuallyDrop::take(this.future.get_unchecked_mut()) };
            if !future.is_terminated() {
                tokio::task::spawn(future);
            }
        }
    }
}

impl<S, F: 'static + Send + Future<Output = ()>> FutureOnCloseStream<S, F> {
    pub fn new(stream: S, future: F) -> Self {
        use futures::FutureExt;
        Self {
            stream,
            future: ManuallyDrop::new(future.fuse()),
        }
    }
}

impl<S, F> Stream for FutureOnCloseStream<S, F>
where
    S: Stream,
    F: Future<Output = ()> + Send + 'static,
{
    type Item = S::Item;

    fn poll_next(
        mut self: std::pin::Pin<&mut Self>,
        cx: &mut std::task::Context<'_>,
    ) -> std::task::Poll<Option<Self::Item>> {
        let this = self.as_mut().project();
        match this.stream.poll_next(cx) {
            std::task::Poll::Ready(Some(item)) => std::task::Poll::Ready(Some(item)),
            std::task::Poll::Ready(None) => {
                let future = unsafe { Pin::new_unchecked(&mut **this.future.get_unchecked_mut()) };
                match future.poll(cx) {
                    std::task::Poll::Ready(_) => std::task::Poll::Ready(None),
                    std::task::Poll::Pending => std::task::Poll::Pending,
                }
            }
            std::task::Poll::Pending => std::task::Poll::Pending,
        }
    }
}
