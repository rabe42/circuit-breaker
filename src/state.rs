use std::fmt::Debug;

///
/// The three states of the CircuitBreaker.
///
#[derive(Debug, PartialEq)]
pub enum CircuitState {
    Open, Close, HalfOpen
}
