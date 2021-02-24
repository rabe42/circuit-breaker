use std::error::Error;
use std::time::{Duration, SystemTime};
use log::{debug, warn, error, trace};

use crate::state::CircuitState;
use crate::CircuitBreaker;
use crate::CircuitBreakerError;

///
/// The CircuitBreaker is implementing the protection pattern for distributed services.
/// It is basically used in my case to protect the service from database failures.
///
pub struct ThresholdBreaker {
    /// The name of this breaker to better identify it in the locks.
    name: String,
    /// The current count of failures. Will be resetted by success.
    failure_count: usize,
    /// The current state of the circuite breaker
    status: CircuitState,
    /// number of exapted failures
    threshold: usize,
    /// timeout to be waited before we try to execute again.
    timeout: Duration,
    /// The point in time, when the circuit was opened.
    time_of_tripping: Option<SystemTime>
}
impl <F, R, E: Error> CircuitBreaker <F, R, E> for ThresholdBreaker
    where F: FnOnce() -> Result<R, E>
{
    /// Try to execute and count the failures here.
    /// Any error returned by the embedded function will be propagated to the callee.
    /// In addition CircuteBreakerError might be thrown.
    fn call(&mut self, f: F) -> Result<R, CircuitBreakerError<E>> {
        debug!("[CircuitBreaker::execute({})]", self.name);
        match self.status {
            CircuitState::Open => self.handle_open(f),
            CircuitState::Close => self.handle_close(f),
            CircuitState::HalfOpen => self.handle_half_open(f)
        }
    }
}
impl ThresholdBreaker
{
    /// Creates a new CircuitBreaker instance.
    /// @param name The name of the circuite breaker, for logging/debugging purposes.
    /// @param function The function, which will be wrapped by the circuit breaker.
    /// @param threshold The number of consecutive failures, which trip the circuit breaker.
    /// @param timeout The time before the circuit breaker isn't changing back to the close status.
    pub fn new(
        name: &str,
        threshold: Option<usize>,
        timeout: Option<Duration>) -> ThresholdBreaker
    {
        debug!("[CircuitBreaker::new({})]", name);

        ThresholdBreaker {
            name: String::from(name),
            failure_count: 0,
            status: CircuitState::Close,
            threshold: if let Some(t) = threshold { t } else { 5 },
            timeout: if let Some(d) = timeout { d } else { Duration::new(5, 0) },
            time_of_tripping: None
        }
    }

    /// Handle the case if the circuit is open (tripped).
    /// It just checks, if the time is up. If not, it just returns an CircuitBreakerError.
    /// Moves to HalfOpen and calling execute otherwise.
    fn handle_open<F, R, E: Error>(&mut self, f: F) -> Result<R, CircuitBreakerError<E>>
        where F: FnOnce() -> Result<R, E>
    {
        debug!("[CircuitBreaker::handle_open({})]", self.name);
        let now = SystemTime::now();
        let time_of_tripping = if let Some(tot) = self.time_of_tripping { tot } else { now };
        if now > time_of_tripping + self.timeout {
            self.status = CircuitState::HalfOpen;
            self.call(f)
        }
        else {
            debug!("[CircuitBreaker::handle_open({})] stays open!", self.name);
            Err(CircuitBreakerError::StaysOpen(String::from(&self.name)))
        }
    }

    /// Handle the case, if the circuit is (still) closed.
    /// In this case it tries to execute the function with the provided parameters.
    /// If this fails, it will increase the failure counter, if the threshold reached,
    /// it will trip().
    fn handle_close<F, R, E: Error>(&mut self, f: F) -> Result<R, CircuitBreakerError<E>>
        where F: FnOnce() -> Result<R, E>
    {
        debug!("[CircuitBreaker::handle_close({})]", self.name);
        match f() {
            Ok(result) => {
                trace!("[CircuitBreaker::handle_close({})] Function called succssfully.", self.name);
                self.reset();
                Ok(result)
            },
            Err(error) => {
                self.failure_count += 1;
                warn!("[CircuitBreaker::handle_close({})] Function call failed {} times.",
                    self.name, self.failure_count);
                if self.failure_count > self.threshold {
                    return self.trip(error);
                }
                Err(CircuitBreakerError::Failed(error))
            }
        }
    }

    /// Handle the HalfOpen state. This is the state, after a Open state.
    /// It executes the function with the provided parameters. If this is successful,
    /// it goes to the close state. It trip() again otherwise.
    fn handle_half_open<F, R, E: Error>(&mut self, f: F) -> Result<R, CircuitBreakerError<E>>
        where F: FnOnce() -> Result<R, E>
    {
        debug!("[CircuitBreaker::handle_half_open({})]", self.name);
        match f() {
            Ok(result) => {
                debug!("[CircuitBreaker::handle_half_open({})] Function called successfully.", self.name);
                self.reset();
                Ok(result)
            }
            Err(error) => {
                warn!("[CircuitBreaker::handle_half_open({})] Still not going to open!", self.name);
                return self.trip(error)
            }
        }
    }

    /// Resetting the failure count and setting the CircuitBreaker in close state.
    fn reset(&mut self) {
        debug!("[CircuitBreaker::reset({})]", self.name);
        self.failure_count = 0;
        self.status = CircuitState::Close;
        self.time_of_tripping = None;
    }

    /// Setting the circuit breaker into the open state.
    fn trip<R, E: Error>(&mut self, error: E) -> Result<R, CircuitBreakerError<E>> {
        error!("[CircuitBreaker::trip({})]", self.name);
        self.status = CircuitState::Open;
        self.time_of_tripping = Some(SystemTime::now());
        Err(CircuitBreakerError::Tripped(String::from(&self.name), error))
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::thread::sleep;
    use log::{debug, error};
    use std::time::Duration;
    use thiserror::Error;

    #[derive(Error, Debug)]
    enum TestError {
        #[error("An expected failure!")]
        ExpectedFailure
    }

    fn success(parameter: &str) -> Result<String, TestError> {
        debug!("[tests::success()] {}", parameter);
        Ok(String::from(parameter))
    }

    fn fail(should_fail: bool) -> Result<&'static str, TestError> {
        match should_fail {
            true => Err(TestError::ExpectedFailure),
            false => Ok("Don't fail")
        }
    }

    #[test]
    fn successful_execute() {
        let mut cb = ThresholdBreaker::new("successful_execute", None, None);
        match cb.call(|| success("Hello")) {
            Ok(msg) => {
                assert_eq!("Hello", msg);
                assert_eq!(CircuitState::Close, cb.status);
            },
            Err(err) => panic!("Unexpected failure: {}!", err)
        }
        match cb.call(|| success("World")) {
            Ok(msg) => assert_eq!("World", msg),
            Err(err) => panic!("Unexpected failure: {}!", err)
        }
    }

    #[test]
    fn unsuccessful_execute() {
        let mut cb = ThresholdBreaker::new("unsuccessful_execute", None, None);
        match cb.call(|| fail(true)) {
            Ok(_) => panic!("Unexpected successful execution!"),
            Err(error) => debug!("Expected error: {}", error)
        }
    }

    #[test]
    fn recover_execute() {
        let mut cb = ThresholdBreaker::new("recover_execute", Some(1), Some(Duration::new(1, 0)));
        // Everything is fine
        match cb.call(|| fail(false)) {
            Ok(_) => assert_eq!(CircuitState::Close, cb.status),
            Err(err) => panic!("Unexpected error: {}", err)
        }
        // One failure is no failure!
        match cb.call(|| fail(true)) {
            Ok(_) => panic!("Unexpected success!"),
            Err(_) => assert_eq!(CircuitState::Close, cb.status)
        }
        // Now the threshold steps in!
        match cb.call(|| fail(true)) {
            Ok(_) => panic!("Unexpected success!"),
            Err(_) => assert_eq!(CircuitState::Open, cb.status)
        }
        // Still in the within the timeout period! The successful function is not even called.
        for _i in 1..10 {
            match cb.call(|| fail(false)) {
                Ok(_) => panic!("Unexpected success!"),
                Err(_) => assert_eq!(CircuitState::Open, cb.status)
            }
        }
        sleep(cb.timeout);
        match cb.call(|| fail(false)) {
            Ok(_) => assert_eq!(CircuitState::Close, cb.status),
            Err(err) => panic!("Unexpected error: {}", err)
        }
    }
}
