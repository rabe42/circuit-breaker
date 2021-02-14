# CircuitBreaker

The implementation of an circuit breaker, you find here, was inspired by the C# implementation of Tim Ross
(https://timross.wordpress.com/2008/02/10/implementing-the-circuit-breaker-pattern-in-c/).
It does not implement the SLA enhancements, described in the article, but sticks to the basic function as described in Release-IT by T. Nygard.

## Overview
The implementation wraps a single function, which might fail. The function can take exactly one parameter of arbitrary type and can return the instance of another type. 

The circuit breaker has three states:
1. Close
2. Open
3. HalfOpen

After the creation, the circuit breaker starts in the closed state. In this state every call to the execute method will call the wrapped function. If the  function fails, the counter of failures will be incremented and the error of the function will be returned. With every successul call, the counter will be resetted to zero. 

If the number of function fails reaches the provided threshold, the circuit breaker will trip to the open state and remains there until the provided timeout duration is over. During this time the wrapped function will not be called on execute, but a CircutiBreakerError will be returned.

The first call after the timeout duration will set the circuit breaker into the half open state. The function will be called and only in the case this call is successful, the counter will be reset to zero and the circuit breaker will be back in the close state.

If the function fails again. The time of the last failure is stored and the function not called before the end of the timeout.

## Logging
The logging is established for this crate, using the log crate. Each circuit breaker gets a name, which is printed also in all log entries.

## Defaults
* The default for the threshold is 5.
* The default of the timeout is 5s.

## Instantiation
To wrap a function with the circuti breaker, you call the new constructor.
``` rust
    let mut cb = CircuitBreaker::new("simple", action, None, None);
```
The first parameter give the circuit breker a name. This will be used in logging. The second parameter is the function, which has to have the folowing signature:
``` rust
    fn (P) -> Result<T, Box<dyn Error>>
```

## Executing the wrapped function.
The circuit breaker manages an internal state. The wrapped function can be simply called like so:
``` rust
    match cb.execute(parameter) {
        Ok(result_of_the_function) => { /* Use the result of the wraped function here*/ },
        Err(error) => { /* One of the error occured might be the one 
            that the circuit breaker tripped. */ }
    }
```

# Example

To run the example enter on the command line:
```
 RUST_LOG=INFO cargo run --example simple
```

You may find a simple example about the use of the circuit breaker in the examples directory:

```
./examples/simple.rs
```

