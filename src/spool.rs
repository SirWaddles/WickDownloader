use std::pin::Pin;
use futures::task::Context;
use futures::{Future, FutureExt, task::Poll};

pub type PinnedBoxFuture<'a, T> = Pin<Box<dyn Future<Output = T> + Send + 'a>>;

pub struct Spool<'a, T, E> {
    futures: Vec<PinnedBoxFuture<'a, Result<T, E>>>,
    active_futures: Vec<PinnedBoxFuture<'a, Result<T, E>>>,
    spool_limit: usize,
}

impl<'a, T, E> Spool<'a, T, E> {
    pub fn build(futures: Vec<PinnedBoxFuture<'a, Result<T, E>>>, spool_limit: usize) -> Spool<'a, T, E> {
        Self {
            futures,
            active_futures: Vec::new(),
            spool_limit,
        }
    }
}

impl<T, E> Future for Spool<'_, T, E> {
    type Output = Result<(), E>;

    fn poll(mut self: Pin<&mut Self>, cx: &mut Context<'_>) -> Poll<Self::Output> {
        let mut i = 0;
        while i < self.active_futures.len() {
            match self.active_futures[i].poll_unpin(cx) {
                Poll::Ready(Ok(_val)) => {
                    self.active_futures.remove(i);
                },
                Poll::Ready(Err(err)) => {
                    return Poll::Ready(Err(err));
                }
                Poll::Pending => {
                    i += 1;
                },
            }
        }

        let new_requests = std::cmp::min(self.futures.len(), self.spool_limit - self.active_futures.len());
        for _ in 0..new_requests {
            let removed = self.futures.remove(0);
            self.active_futures.push(removed);
        }

        if self.active_futures.len() <= 0 && self.futures.len() <= 0 {
            return Poll::Ready(Ok(()));
        }
        Poll::Pending
    }
}
