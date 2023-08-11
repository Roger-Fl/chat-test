use netutils::{thread_helper, message_stream::MsgError};
use std::error::Error;

#[derive(Debug)]
pub enum ClientError {
    IoError(std::io::Error),
    MsgError(MsgError),
    ThreadError(thread_helper::ThreadError)
}

impl std::fmt::Display for ClientError {
    fn fmt(&self, f: &mut std::fmt::Formatter) -> std::fmt::Result {
        match &self {
            ClientError::IoError(e) => write!(f, "IoError: {}, kind: {}", e, e.kind()),
            ClientError::MsgError(e) => write!(f, "MsgError: {}", e),
            ClientError::ThreadError(e) => write!(f, "ThreadError: {}", e),
        }
    }
}

impl Error for ClientError {
    fn source(&self) -> Option<&(dyn Error + 'static)> {
        None
    }
    fn cause(&self) -> Option<&dyn Error> {
        None
    }
}

impl From<std::io::Error> for ClientError {
    fn from(e: std::io::Error) -> Self {
        Self::IoError(e)
    }
}
impl From<MsgError> for ClientError {
    fn from(e: MsgError) -> Self {
        Self::MsgError(e)
    }
}
impl From<thread_helper::ThreadError> for ClientError {
    fn from(e: thread_helper::ThreadError) -> Self {
        Self::ThreadError(e)
    }
}