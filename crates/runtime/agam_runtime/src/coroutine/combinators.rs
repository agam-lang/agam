//! Async Future Combinators (Select, Join, Race).

use std::future::Future as StdFuture;
use std::pin::Pin;
use std::task::{Context as StdContext, Poll as StdPoll};

/// Sum type representing the winner of a `select` operation.
#[derive(Clone, Debug, PartialEq, Eq)]
pub enum Either<A, B> {
    Left(A),
    Right(B),
}

/// Race two futures concurrently, returning the output of whichever completes first.
pub async fn select<F1, F2>(fut1: F1, fut2: F2) -> Either<F1::Output, F2::Output>
where
    F1: StdFuture,
    F2: StdFuture,
{
    struct SelectFut<F1, F2> {
        pin1: Pin<Box<F1>>,
        pin2: Pin<Box<F2>>,
    }

    impl<F1: StdFuture, F2: StdFuture> StdFuture for SelectFut<F1, F2> {
        type Output = Either<F1::Output, F2::Output>;

        fn poll(self: Pin<&mut Self>, cx: &mut StdContext<'_>) -> StdPoll<Self::Output> {
            let this = unsafe { self.get_unchecked_mut() };
            if let StdPoll::Ready(v1) = this.pin1.as_mut().poll(cx) {
                return StdPoll::Ready(Either::Left(v1));
            }

            if let StdPoll::Ready(v2) = this.pin2.as_mut().poll(cx) {
                return StdPoll::Ready(Either::Right(v2));
            }

            StdPoll::Pending
        }
    }

    SelectFut {
        pin1: Box::pin(fut1),
        pin2: Box::pin(fut2),
    }
    .await
}

/// Await two futures concurrently and return both outputs once both finish.
pub async fn join<F1, F2>(fut1: F1, fut2: F2) -> (F1::Output, F2::Output)
where
    F1: StdFuture,
    F2: StdFuture,
    F1::Output: Clone,
    F2::Output: Clone,
{
    struct JoinFut<F1: StdFuture, F2: StdFuture> {
        pin1: Pin<Box<F1>>,
        pin2: Pin<Box<F2>>,
        res1: Option<F1::Output>,
        res2: Option<F2::Output>,
    }

    impl<F1, F2> StdFuture for JoinFut<F1, F2>
    where
        F1: StdFuture,
        F2: StdFuture,
        F1::Output: Clone,
        F2::Output: Clone,
    {
        type Output = (F1::Output, F2::Output);

        fn poll(self: Pin<&mut Self>, cx: &mut StdContext<'_>) -> StdPoll<Self::Output> {
            let this = unsafe { self.get_unchecked_mut() };
            if this.res1.is_none() {
                let poll1 = this.pin1.as_mut().poll(cx);
                if let StdPoll::Ready(val) = poll1 {
                    this.res1 = Some(val);
                }
            }

            if this.res2.is_none() {
                let poll2 = this.pin2.as_mut().poll(cx);
                if let StdPoll::Ready(val) = poll2 {
                    this.res2 = Some(val);
                }
            }

            if this.res1.is_some() && this.res2.is_some() {
                StdPoll::Ready((this.res1.clone().unwrap(), this.res2.clone().unwrap()))
            } else {
                StdPoll::Pending
            }
        }
    }

    JoinFut {
        pin1: Box::pin(fut1),
        pin2: Box::pin(fut2),
        res1: None,
        res2: None,
    }
    .await
}
