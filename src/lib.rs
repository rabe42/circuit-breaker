use std::error::Error;
use std::time::{Duration, SystemTime};
use log::{debug, warn, info, error, trace};
use thiserror::Error;

///
/// The error object, returned, if the circuit breaker has to do its job.
///
#[derive(Error, Debug)]
pub enum CircuitBreakerError {
    #[error("The circuit breaker '{0}' will stay open.")]
    StaysOpen(String)
}

///
/// The three states of the CircuitBreaker.
///
#[derive(Debug, PartialEq)]
enum CircuitState {
    Open, Close, HalfOpen
}

///
/// The CircuitBreaker is implementing the protection pattern for distributed services.
/// It is basically used in my case to protect the service from database failures.
///
pub struct ThresholdBreaker<P, T> {
    /// The name of this breaker to better identify it in the locks.
    name: String,
    /// The function to be executed.
    function: fn(P) -> Result<T, Box<dyn Error>>,
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
impl <P, T> ThresholdBreaker<P, T> {
    /// Creates a new CircuitBreaker instance.
    /// @param name The name of the circuite breaker, for logging/debugging purposes.
    /// @param function The function, which will be wrapped by the circuit breaker.
    /// @param threshold The number of consecutive failures, which trip the circuit breaker.
    /// @param timeout The time before the circuit breaker isn't changing back to the close status.
    pub fn new(
        name: &str,
        function: fn(P) -> Result<T, Box<dyn Error>>,
        threshold: Option<usize>,
        timeout: Option<Duration>) -> ThresholdBreaker<P, T> {

        debug!("[CircuitBreaker::new({})]", name);

        ThresholdBreaker {
            name: String::from(name),
            function,
            failure_count: 0,
            status: CircuitState::Close,
            threshold: if let Some(t) = threshold { t } else { 5 },
            timeout: if let Some(d) = timeout { d } else { Duration::new(5, 0) },
            time_of_tripping: None
        }
    }

    /// Try to execute and count the failures here.
    /// Any error returned by the embedded function will be propagated to the callee.
    /// In addition CircuteBreakerError might be thrown.
    pub fn execute(&mut self, parameter: P) -> Result<T, Box<dyn Error>> {
        debug!("[CircuitBreaker::execute({})]", self.name);
        match self.status {
            CircuitState::Open => self.handle_open(parameter),
            CircuitState::Close => self.handle_close(parameter),
            CircuitState::HalfOpen => self.handle_half_open(parameter)
        }
    }

    /// Handle the case if the circuit is open (tripped).
    /// It just checks, if the time is up. If not, it just returns an CircuitBreakerError.
    /// Moves to HalfOpen and calling execute otherwise.
    fn handle_open(&mut self, parameter: P) -> Result<T, Box<dyn Error>> {
        debug!("[CircuitBreaker::handle_open({})]", self.name);
        let now = SystemTime::now();
        let time_of_tripping = if let Some(tot) = self.time_of_tripping { tot } else { now };
        if now > time_of_tripping + self.timeout {
            self.status = CircuitState::HalfOpen;
            self.execute(parameter)
        }
        else {
            debug!("[CircuitBreaker::handle_open({})] stays open!", self.name);
            Err(Box::new(CircuitBreakerError::StaysOpen(String::from(&self.name))))
        }
    }

    /// Handle the case, if the circuit is (still) closed.
    /// In this case it tries to execute the function with the provided parameters.
    /// If this fails, it will increase the failure counter, if the threshold reached,
    /// it will trip().
    fn handle_close(&mut self, parameter: P) -> Result<T, Box<dyn Error>> {
        debug!("[CircuitBreaker::handle_close({})]", self.name);
        match (self.function)(parameter) {
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
                    self.trip();
                }
                Err(error)
            }
        }
    }

    /// Handle the HalfOpen state. This is the state, after a Open state.
    /// It executes the function with the provided parameters. If this is successful,
    /// it goes to the close state. It trip() again otherwise.
    fn handle_half_open(&mut self, parameter: P) -> Result<T, Box<dyn Error>> {
        debug!("[CircuitBreaker::handle_half_open({})]", self.name);
        match (self.function)(parameter) {
            Ok(result) => {
                info!("[CircuitBreaker::handle_half_open({})] Function called successfully.", self.name);
                self.reset();
                Ok(result)
            }
            Err(error) => {
                warn!("[CircuitBreaker::handle_half_open({})] Still not going to open!", self.name);
                self.trip();
                Err(error)
            }
        }
    }

    /// Resetting the failure count and setting the CircuitBreaker in close state.
    fn reset(&mut self) {
        info!("[CircuitBreaker::reset({})]", self.name);
        self.failure_count = 0;
        self.status = CircuitState::Close;
        self.time_of_tripping = None;
    }

    /// Setting the circuit breaker into the open state.
    fn trip(&mut self) {
        error!("[CircuitBreaker::trip({})]", self.name);
        self.status = CircuitState::Open;
        self.time_of_tripping = Some(SystemTime::now());
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::thread::sleep;

    #[derive(Error, Debug)]
    enum TestError {
        #[error("An expected failure!")]
        ExpectedFailure
    }

    fn success(parameter: &str) -> Result<String, Box<dyn Error>> {
        debug!("[tests::success()] {}", parameter);
        Ok(String::from(parameter))
    }

    fn fail(should_fail: bool) -> Result<&'static str, Box<dyn Error>> {
        match should_fail {
            true => Err(Box::new(TestError::ExpectedFailure)),
            false => Ok("Don't fail")
        }
    }

    #[test]
    fn successful_execute() {
        let mut cb = ThresholdBreaker::new("successful_execute", success, None, None);
        match cb.execute("Hello") {
            Ok(msg) => {
                assert_eq!("Hello", msg);
                assert_eq!(CircuitState::Close, cb.status);
            },
            Err(err) => panic!("Unexpected failure: {}!", err)
        }
        match cb.execute("World") {
            Ok(msg) => assert_eq!("World", msg),
            Err(err) => panic!("Unexpected failure: {}!", err)
        }
    }

    #[test]
    fn unsuccessful_execute() {
        let mut cb = ThresholdBreaker::new("unsuccessful_execute", fail, None, None);
        match cb.execute(true) {
            Ok(_) => panic!("Unexpected successful execution!"),
            //Err(TestError::ExpectedFailure) => debug!("Expected failure!"),
            Err(error) => debug!("Expected error: {}", error)
        }
    }

    #[test]
    fn recover_execute() {
        let mut cb = ThresholdBreaker::new("recover_execute", fail, Some(1), Some(Duration::new(1, 0)));
        // Everything is fine
        match cb.execute(false) {
            Ok(_) => assert_eq!(CircuitState::Close, cb.status),
            Err(err) => panic!("Unexpected error: {}", err)
        }
        // One failure is no failure!
        match cb.execute(true) {
            Ok(_) => panic!("Unexpected success!"),
            Err(_) => assert_eq!(CircuitState::Close, cb.status)
        }
        // Now the threshold steps in!
        match cb.execute(true) {
            Ok(_) => panic!("Unexpected success!"),
            Err(_) => assert_eq!(CircuitState::Open, cb.status)
        }
        // Still in the within the timeout period! The successful function is not even called.
        for _i in 1..10 {
            match cb.execute(false) {
                Ok(_) => panic!("Unexpected success!"),
                Err(_) => assert_eq!(CircuitState::Open, cb.status)
            }
        }
        sleep(cb.timeout);
        match cb.execute(false) {
            Ok(_) => assert_eq!(CircuitState::Close, cb.status),
            Err(err) => panic!("Unexpected error: {}", err)
        }
    }
}
