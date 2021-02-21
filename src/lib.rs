use std::error::Error;

mod error;
mod state;
mod threshold;

pub use error::CircuitBreakerError;
pub use threshold::ThresholdBreaker;

///
/// This is the trait, which implements the call to the wrapped function.
/// 
pub trait CircuitBreaker<P, R, E: Error> {
    fn call(&mut self, parameter: P) -> Result<R, CircuitBreakerError<E>>;
}
