use env_logger;
use circuit_breaker::ThresholdBreaker;
use log::{info};
use std::time::Duration;
use thiserror::Error;
use std::thread::sleep;

/// A Simple Error definition.
#[derive(Error, Debug)]
enum ActionError {
    #[error("Failed Action")]
    Fail
}


/// An action doing normally something, which might fail.
fn action(should_fail: bool) -> Result<(), ActionError> {
    match should_fail {
        true => Err(ActionError::Fail),
        false => Ok(())
    }
}

fn main() {
    env_logger::init();

    let mut cb = ThresholdBreaker::new("simple", action, Some(1), Some(Duration::new(1, 0)));
    // Everything is fine
    match cb.execute(false) {
        Ok(_) => info!("Everything is fine!"),
        Err(err) => panic!("Unexpected error: {}", err)
    }
    // One failure is no failure!
    match cb.execute(true) {
        Ok(_) => panic!("Unexpected success!"),
        Err(_) => info!("First error on execute!")
    }
    // Now the threshold steps in!
    match cb.execute(true) {
        Ok(_) => panic!("Unexpected success!"),
        Err(_) => info!("Second error, circuit should be open now!")
    }
    // Still in the within the timeout period! The successful function is not even called.
    for _i in 1..10 {
        match cb.execute(false) {
            Ok(_) => panic!("Unexpected success!"),
            Err(_) => info!("Even the execution is now successful, we're still in open and get an error!")
        }
    }
    sleep(Duration::new(1,0));
    match cb.execute(false) {
        Ok(_) => info!("Now everything is back to normal!"),
        Err(err) => panic!("Unexpected error: {}", err)
    }
}
