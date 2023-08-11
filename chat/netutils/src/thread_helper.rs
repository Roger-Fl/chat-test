use std::thread;
use std::sync::atomic::AtomicBool;
use std::error::Error;
use std::sync::Arc;

use crate::logger::Logger;

pub struct ThreadHelper {
    thread_state: Arc<ThreadState>,
    name: String,
    handle: Option<thread::JoinHandle<()>>,
    logger: Arc<Logger>,
}

pub struct ThreadState {
    shutting_down: Arc<AtomicBool>,
}

#[derive(Debug)]
pub enum ThreadError {
    AlreadyShuttingDown,
    ThreadError,
}

impl Error for ThreadError {
    fn source(&self) -> Option<&(dyn Error + 'static)> {
        None
    }
    fn cause(&self) -> Option<&dyn Error> {
        None
    }
}

impl std::fmt::Display for ThreadError {
    fn fmt(&self, f: &mut std::fmt::Formatter) -> Result<(), std::fmt::Error> {
        match &self {
            ThreadError::AlreadyShuttingDown => {
                write!(f, "Thread is already shutting down")
            },
            ThreadError::ThreadError => {
                write!(f, "Thread error")
            },
        }
    }
}

impl ThreadHelper {
    pub fn new(thread_state: Arc<ThreadState>, logger: Arc<Logger>) -> Self {
        ThreadHelper {
            thread_state,
            name: String::new(),
            handle: None,
            logger: logger,
        }
    }

    pub fn start<Job>(&mut self, name: String, job: Job)
    where
        Job: FnOnce() + Send + 'static, 
    {
        let thread = thread::spawn(move || job());

        self.name = name;
        self.handle = Some(thread);
    }

    pub fn shutdown(&mut self) -> Result<(), ThreadError> {
        self.logger.log(format!("Thread {} shutting down...", self.name).as_bytes()).expect("Failed to write to log");
        // let f = |writer: &mut Box<dyn std::io::Write>| writer.write(format!("Thread {} shutting down...", self.name).as_bytes()).expect("Failed to write to log");
        // self.logger.apply(f);

        self.thread_state.shutdown_start()?;
        self.wait_for_shutdown();
        Ok(())
    }

    pub fn wait_for_shutdown(&mut self) {
        if let Some(handle) = self.handle.take() {
            handle.join().expect("Failed to join thread"); //join requires a move (takes self instead of &self)
        }

        self.logger.log(format!("Thread {} shut down successfully", self.name).as_bytes()).expect("Failed to write to log");
    }
}

impl Drop for ThreadHelper {
    fn drop(&mut self) {
        let res = self.shutdown();
        match res {
            Ok(_) => {},
            Err(ThreadError::AlreadyShuttingDown) => {},
            Err(_) => panic!("Thread error"),
        }
    }
}

impl ThreadState {
    pub fn new() -> Self {
        ThreadState {
            shutting_down: Arc::new(AtomicBool::new(false)),
        }
    }

    pub fn shutdown_start(&self) -> Result<(), ThreadError> {
        if self.is_shuttingdown() {
            return Err(ThreadError::AlreadyShuttingDown);
        }

        self.shutting_down.store(true, std::sync::atomic::Ordering::SeqCst);
        Ok(())
    }

    pub fn is_shuttingdown(&self) -> bool {
        self.shutting_down.load(std::sync::atomic::Ordering::SeqCst)
    }
}