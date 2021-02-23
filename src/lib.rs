use std::error::Error;

mod error;
mod state;
mod threshold;

pub use error::CircuitBreakerError;
pub use threshold::ThresholdBreaker;

//pub type Callback = FnOnce() -> Result<_, E = Error>;

///
/// This is the trait, which implements the call to the wrapped function.
/// 
pub trait CircuitBreaker<F, R, E: Error> 
    where F: FnOnce() -> Result<R, E>
{
    fn call(&mut self, f: F) -> Result<R, CircuitBreakerError<E>>;
}
