use netutils::{thread_helper, message_stream::MsgError};
use std::error::Error;

#[derive(Debug)]
pub enum ServerError {
    IoError(std::io::Error),
    MsgError(MsgError),
    ThreadError(thread_helper::ThreadError),
    ReadError(String),
}

impl std::fmt::Display for ServerError {
    fn fmt(&self, f: &mut std::fmt::Formatter) -> std::fmt::Result {
        match &self {
            ServerError::IoError(e) => write!(f, "IoError: {}, kind: {}", e, e.kind()),
            ServerError::MsgError(e) => write!(f, "MsgError: {}", e),
            ServerError::ThreadError(e) => write!(f, "ThreadError: {}", e),
            ServerError::ReadError(e) => write!(f, "ReadError: {}", e),
        }
    }
}

impl Error for ServerError {
    fn source(&self) -> Option<&(dyn Error + 'static)> {
        None
    }
    fn cause(&self) -> Option<&dyn Error> {
        None
    }
}

impl From<std::io::Error> for ServerError {
    fn from(e: std::io::Error) -> Self {
        Self::IoError(e)
    }
}
impl From<MsgError> for ServerError {
    fn from(e: MsgError) -> Self {
        Self::MsgError(e)
    }
}
impl From<thread_helper::ThreadError> for ServerError {
    fn from(e: thread_helper::ThreadError) -> Self {
        Self::ThreadError(e)
    }
}