use log::error;
use std::error::Error;
use thiserror::Error;

///
/// The error object, returned, if the circuit breaker has to do its job.
///
#[derive(Error, Debug)]
pub enum CircuitBreakerError<E: Error> {
    /// Returned, if the wrapped function failed with an generic error.
    /// This error should be extracted, if of interest.
    #[error("The wrapped function failed with.")]
    Failed(E),
    /// The name of the circuit breaker can be extracted from this error. It is returned,
    /// if the circuit breaker opens the connection.
    #[error("The circuit breaker '{0}' tripped to open.")]
    Tripped(String, E),
    /// The name of the circuit breaker can be extracted from this error. It is returned,
    /// if the circuit breaker stays open.
    #[error("The circuit breaker '{0}' will stay open.")]
    StaysOpen(String)
}

