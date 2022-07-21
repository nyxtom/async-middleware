//! Middleware types.

use async_trait::async_trait;
use std::{future::Future, marker::PhantomData, sync::Arc};

/// Middleware that transforms around an input to output type.
#[async_trait]
pub trait Transform<Args, T, O>: Send + Sync + 'static {
    /// Asynchronously execute this handler to modify state
    async fn transform(&self, input: T) -> O;
}

/// Middleware implementation for an async function that produces an output
#[async_trait]
impl<Func, Fut, O> Transform<(), (), O> for Func
where
    Func: Send + Sync + 'static + Fn() -> Fut,
    Fut: Future<Output = O> + Send + Sync + 'static,
    O: Send + Sync + 'static,
{
    async fn transform(&self, _input: ()) -> O {
        (self)().await
    }
}

/// Middleware implementation for an async function that returns nothing
#[async_trait]
impl<Func, Fut, T, O> Transform<(T, O), T, O> for Func
where
    Func: Send + Sync + 'static + Fn(T) -> Fut,
    Fut: Future<Output = O> + Send + Sync + 'static,
    T: Send + Sync + 'static,
    O: Send + Sync + 'static,
{
    async fn transform(&self, input: T) -> O {
        (self)(input).await
    }
}

/// Middleware that performs an operation.
#[async_trait]
pub trait Middleware<I, O>: Send + Sync + 'static {
    async fn call(&self, input: I) -> O;
}

/// Encapsulates the conversion between two different transform types
pub struct ConvertMiddleware<T, T2, A, B, C> {
    t: Arc<dyn Transform<T, A, B>>,
    t2: Arc<dyn Transform<T2, B, C>>,
}

/// Implements the transform trait on the conversion middleware (for downstream)
#[async_trait]
impl<T, T2, A, B, C> Transform<(A, C), A, C> for ConvertMiddleware<T, T2, A, B, C>
where
    T: Send + Sync + 'static,
    T2: Send + Sync + 'static,
    A: Send + Sync + 'static,
    B: Send + Sync + 'static,
    C: Send + Sync + 'static,
{
    async fn transform(&self, input: A) -> C {
        let input = self.t.transform(input).await;
        self.t2.transform(input).await
    }
}

/// Implements the middleware trait on the conversion middleware to make it A -> C
#[async_trait]
impl<T, T2, A, B, C> Middleware<A, C> for ConvertMiddleware<T, T2, A, B, C>
where
    T: Send + Sync + 'static,
    T2: Send + Sync + 'static,
    A: Send + Sync + 'static,
    B: Send + Sync + 'static,
    C: Send + Sync + 'static,
{
    async fn call(&self, input: A) -> C {
        self.transform(input).await
    }
}

/// Creates a new conversion middleware from two existing transforms
pub fn convert<T, T2, A, B, C>(
    t: impl Transform<T, A, B>,
    t2: impl Transform<T2, B, C>,
) -> ConvertMiddleware<T, T2, A, B, C>
where
    T: Send + Sync + 'static,
    T2: Send + Sync + 'static,
    A: Send + Sync + 'static,
    B: Send + Sync + 'static,
    C: Send + Sync + 'static,
{
    ConvertMiddleware {
        t: Arc::new(t),
        t2: Arc::new(t2),
    }
}

/// Pied constructs the way we pipe between lots of functions via middleware
pub struct Pied<T, Args, I, O> {
    middleware: Arc<dyn Middleware<I, O>>,
    _phantom: PhantomData<T>,
    _phantom2: PhantomData<Args>,
}

/// Implements the middleware trait for the main Pied structure
#[async_trait]
impl<T, Args, I, O> Middleware<I, O> for Pied<T, Args, I, O>
where
    T: Send + Sync + 'static,
    Args: Send + Sync + 'static,
    I: Send + Sync + 'static,
    O: Send + Sync + 'static,
{
    async fn call(&self, input: I) -> O {
        self.middleware.call(input).await
    }
}

/// Common pipe trait used to create implementations for each tuple
pub trait Piper<T, Args, I, O> {
    fn pipe(self) -> Pied<T, Args, I, O>;
}

/// Helper utility to execute the .pipe on a Pipe implementation and returns a middleware
pub fn pipe<T, Args, I, O>(f: impl Piper<T, Args, I, O>) -> impl Middleware<I, O>
where
    T: Send + Sync + 'static,
    Args: Send + Sync + 'static,
    I: Send + Sync + 'static,
    O: Send + Sync + 'static,
{
    f.pipe()
}

// Pipe middleware for source -> transform from (A, B)
impl<T, O, A, B> Piper<(T, O), (A, B), (), O> for (A, B)
where
    A: Transform<(), (), T>,
    B: Transform<(T, O), T, O>,
    T: Send + Sync + 'static,
    O: Send + Sync + 'static,
{
    fn pipe(self) -> Pied<(T, O), (A, B), (), O> {
        let args = self;
        Pied {
            middleware: Arc::new(convert(args.0, args.1)),
            _phantom: PhantomData::default(),
            _phantom2: PhantomData::default(),
        }
    }
}

// Pipe middleware for transform -> transform from (A, B)
impl<T, T2, O, A, B> Piper<(T, T2, O), (A, B), T, O> for (A, B)
where
    A: Transform<(T, T2), T, T2>,
    B: Transform<(T2, O), T2, O>,
    T: Send + Sync + 'static,
    T2: Send + Sync + 'static,
    O: Send + Sync + 'static,
{
    fn pipe(self) -> Pied<(T, T2, O), (A, B), T, O> {
        let args = self;
        Pied {
            middleware: Arc::new(convert(args.0, args.1)),
            _phantom: PhantomData::default(),
            _phantom2: PhantomData::default(),
        }
    }
}

// Pipe middleware for source -> transform -> transform for (A, B, C)
impl<T, T2, O, A, B, C> Piper<(T, T2, O), (A, B, C), (), O> for (A, B, C)
where
    A: Transform<(), (), T>,
    B: Transform<(T, T2), T, T2>,
    C: Transform<(T2, O), T2, O>,
    T: Send + Sync + 'static,
    T2: Send + Sync + 'static,
    O: Send + Sync + 'static,
{
    fn pipe(self) -> Pied<(T, T2, O), (A, B, C), (), O> {
        let args = self;
        Pied {
            middleware: Arc::new(convert(convert(args.0, args.1), args.2)),
            _phantom: PhantomData::default(),
            _phantom2: PhantomData::default(),
        }
    }
}

// Pipe middleware for transform -> transform -> transform for (A, B, C)
impl<T, T2, T3, O, A, B, C> Piper<(T, T2, T3, O), (A, B, C), T, O> for (A, B, C)
where
    A: Transform<(T, T2), T, T2>,
    B: Transform<(T2, T3), T2, T3>,
    C: Transform<(T3, O), T3, O>,
    T: Send + Sync + 'static,
    T2: Send + Sync + 'static,
    T3: Send + Sync + 'static,
    O: Send + Sync + 'static,
{
    fn pipe(self) -> Pied<(T, T2, T3, O), (A, B, C), T, O> {
        let args = self;
        Pied {
            middleware: Arc::new(convert(convert(args.0, args.1), args.2)),
            _phantom: PhantomData::default(),
            _phantom2: PhantomData::default(),
        }
    }
}

// Pipe middleware for source -> transform -> transform -> transform for (A, B, C, D)
impl<T, T2, T3, O, A, B, C, D> Piper<(T, T2, T3, O), (A, B, C, D), (), O> for (A, B, C, D)
where
    A: Transform<(), (), T>,
    B: Transform<(T, T2), T, T2>,
    C: Transform<(T2, T3), T2, T3>,
    D: Transform<(T3, O), T3, O>,
    T: Send + Sync + 'static,
    T2: Send + Sync + 'static,
    T3: Send + Sync + 'static,
    O: Send + Sync + 'static,
{
    fn pipe(self) -> Pied<(T, T2, T3, O), (A, B, C, D), (), O> {
        let args = self;
        Pied {
            middleware: Arc::new(convert(convert(convert(args.0, args.1), args.2), args.3)),
            _phantom: PhantomData::default(),
            _phantom2: PhantomData::default(),
        }
    }
}

// Pipe middleware for transform -> transform -> transform -> transform for (A, B, C, D)
impl<T, T2, T3, T4, O, A, B, C, D> Piper<(T, T2, T3, T4, O), (A, B, C, D), T, O> for (A, B, C, D)
where
    A: Transform<(T, T2), T, T2>,
    B: Transform<(T2, T3), T2, T3>,
    C: Transform<(T3, T4), T3, T4>,
    D: Transform<(T4, O), T4, O>,
    T: Send + Sync + 'static,
    T2: Send + Sync + 'static,
    T3: Send + Sync + 'static,
    T4: Send + Sync + 'static,
    O: Send + Sync + 'static,
{
    fn pipe(self) -> Pied<(T, T2, T3, T4, O), (A, B, C, D), T, O> {
        let args = self;
        Pied {
            middleware: Arc::new(convert(convert(convert(args.0, args.1), args.2), args.3)),
            _phantom: PhantomData::default(),
            _phantom2: PhantomData::default(),
        }
    }
}

// Pipe middleware for source -> transform -> transform -> transform -> transform for (A, B, C, D, E)
impl<T, T2, T3, T4, O, A, B, C, D, E> Piper<(T, T2, T3, T4, O), (A, B, C, D, E), (), O>
    for (A, B, C, D, E)
where
    A: Transform<(), (), T>,
    B: Transform<(T, T2), T, T2>,
    C: Transform<(T2, T3), T2, T3>,
    D: Transform<(T3, T4), T3, T4>,
    E: Transform<(T4, O), T4, O>,
    T: Send + Sync + 'static,
    T2: Send + Sync + 'static,
    T3: Send + Sync + 'static,
    T4: Send + Sync + 'static,
    O: Send + Sync + 'static,
{
    fn pipe(self) -> Pied<(T, T2, T3, T4, O), (A, B, C, D, E), (), O> {
        let args = self;
        Pied {
            middleware: Arc::new(convert(
                convert(convert(convert(args.0, args.1), args.2), args.3),
                args.4,
            )),
            _phantom: PhantomData::default(),
            _phantom2: PhantomData::default(),
        }
    }
}

// Pipe middleware for transform -> transform -> transform -> transform -> transform for (A, B, C, D, E)
impl<T, T2, T3, T4, T5, O, A, B, C, D, E> Piper<(T, T2, T3, T4, T5, O), (A, B, C, D, E), T, O>
    for (A, B, C, D, E)
where
    A: Transform<(T, T2), T, T2>,
    B: Transform<(T2, T3), T2, T3>,
    C: Transform<(T3, T4), T3, T4>,
    D: Transform<(T4, T5), T4, T5>,
    E: Transform<(T5, O), T5, O>,
    T: Send + Sync + 'static,
    T2: Send + Sync + 'static,
    T3: Send + Sync + 'static,
    T4: Send + Sync + 'static,
    T5: Send + Sync + 'static,
    O: Send + Sync + 'static,
{
    fn pipe(self) -> Pied<(T, T2, T3, T4, T5, O), (A, B, C, D, E), T, O> {
        let args = self;
        Pied {
            middleware: Arc::new(convert(
                convert(convert(convert(args.0, args.1), args.2), args.3),
                args.4,
            )),
            _phantom: PhantomData::default(),
            _phantom2: PhantomData::default(),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    async fn producer() -> i32 {
        3
    }

    async fn multipler(i: i32) -> i32 {
        i * 32
    }

    async fn stringer(i: i32) -> String {
        i.to_string()
    }

    async fn logger(s: String) {
        println!("{}", s);
    }

    async fn log_nums(i: i32) {
        println!("{}", i);
    }

    #[test]
    fn test_piper_tuple() {
        pipe((producer, log_nums));
        (producer, log_nums).pipe();
        pipe((producer, stringer, logger));
        (producer, stringer, logger).pipe();
        pipe((producer, multipler, stringer, logger));
        (producer, multipler, stringer, logger).pipe();
        pipe((multipler, multipler, multipler));
        (multipler, multipler, multipler).pipe();
        pipe((multipler, multipler, stringer));
        (multipler, multipler, stringer).pipe();
    }

    #[test]
    fn test_convert_transform() {
        convert(multipler, stringer);
        convert(multipler, multipler);
    }

    #[test]
    fn test_source_transform() {
        convert(producer, multipler);
    }

    #[test]
    fn test_source_sink() {
        convert(producer, log_nums);
    }

    #[test]
    fn test_transform() {
        convert(convert(producer, multipler), stringer);
    }

    #[test]
    fn test_transform_source_transform_sink() {
        convert(convert(convert(producer, multipler), stringer), logger);
    }
}
