use std::pin::Pin;
use futures::task::Context;
use futures::{Future, task::Poll};

pub struct Spool<T> {
    futures: Vec<Pin<Box<T>>>,
    active_futures: Vec<Pin<Box<T>>>,
    spool_limit: usize,
}

impl<I, E, T: Future<Output=Result<I, E>>> Spool<T> {
    pub fn build(futures: Vec<T>, spool_limit: usize) -> impl Future<Output = Result<(), E>> {
        Self {
            futures: futures.into_iter().map(|v| Box::pin(v)).collect(),
            active_futures: Vec::new(),
            spool_limit,
        }
    }
}

impl<I, E, T: Future<Output=Result<I, E>>> Future for Spool<T> {
    type Output = Result<(), E>;

    fn poll(mut self: Pin<&mut Self>, cx: &mut Context) -> Poll<Self::Output> {
        let mut i = 0;
        while i < self.active_futures.len() {
            let active_box = &mut self.active_futures[i];
            match active_box.as_mut().poll(cx) {
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
