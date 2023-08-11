
use std::{
    net::{SocketAddr, TcpStream},
    sync::{Mutex},
};
use core::cell::RefCell;

pub struct ClientStream {
    pub stream_write: Mutex<Option<TcpStream>>,
    pub addr: SocketAddr,
    pub(crate) stream_read: RefCell<Option<TcpStream>>,
}

impl ClientStream {
    pub(crate) fn new(stream: TcpStream) -> Self {
        let stream_read = stream.try_clone().expect("Failed to clone TcpStream");
        let addr = stream.peer_addr().expect("Unable to get peer_addr");
        ClientStream {
            stream_write:Mutex::new(Some(stream)),
            addr:addr,
            stream_read:RefCell::new(Some(stream_read)),
        }
    }
}

pub(crate) struct ClientData {
   pub(crate) msg: Vec::<u8>,
}

impl ClientData {
    pub(crate) fn new() -> Self {
        ClientData {
            msg:Vec::new(),
        }
    }
}