#[macro_use]
extern crate lazy_static;

use crossbeam_channel::{Sender, Receiver, unbounded};

lazy_static! {
    static ref LOG_SENDER: Sender<String> = create_logging_thread();
}

fn create_logging_thread() -> Sender<String>{
    let (logging_tx, logging_rx): (Sender<String>, Receiver<String>) = unbounded();
    std::thread::spawn(move|| { 
        loop {
            match logging_rx.recv() {
                Ok(message) => println!("{}", message),
                Err(_) => {},
            }
        }
    });
    return logging_tx;
}

pub fn pipe_to_output(message: String) {
    LOG_SENDER.send(message);
}

#[macro_export]
macro_rules! info{
    ($($arg:tt)*) => {
        match false {
            true => {(logging::pipe_to_output(format!("[{}] [INFO] {}", std::time::SystemTime::now().duration_since(std::time::SystemTime::UNIX_EPOCH).unwrap().as_millis(), format!($($arg)*).to_string())));},
            false => {println!("{}", format!($($arg)*).to_string())}
        }
    }
}

#[macro_export]
macro_rules! debug{
    ($($arg:tt)*) => {
        match false {
            true => {(logging::pipe_to_output(format!("[{}] [DBUG] {}", std::time::SystemTime::now().duration_since(std::time::SystemTime::UNIX_EPOCH).unwrap().as_millis(), format!($($arg)*).to_string())));},
            false => {println!("debug {}", format!($($arg)*).to_string())}
        }
    }
}