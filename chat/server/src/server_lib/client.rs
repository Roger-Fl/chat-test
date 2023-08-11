
use std::{
    net::{TcpStream, SocketAddr},
    sync::{Arc, Mutex,},
    hash::{Hash, Hasher},
};
use core::cell::RefCell;

pub(crate) struct ClientData {
    pub(crate) msg: Vec::<u8>,
}

impl ClientData {
    fn new() -> Self {
        ClientData {
            msg: Vec::new(),
        }
    }
}


pub struct Client {
    pub stream: Arc<ClientStream>,
    pub(crate) data: RefCell<ClientData>,
}

unsafe impl Sync for Client {}
unsafe impl Send for Client {}

impl PartialEq for Client {
    fn eq(&self, other: &Self) -> bool {
        self.stream.addr == other.stream.addr
    }
}

impl Eq for Client {}

impl Hash for Client {
    fn hash<H: Hasher>(&self, state: &mut H) {
        self.stream.addr.hash(state);
    }
}

pub struct ClientStream {
    pub stream_write: Mutex<TcpStream>,
    pub addr: SocketAddr,
    pub(crate) stream_read: RefCell<TcpStream>,
}

impl ClientStream {
    fn new(stream: TcpStream, addr: SocketAddr) -> Self {
        stream.set_nonblocking(true).expect("Failed to put socket in nonblocking mode");
        let stream_read = stream.try_clone().expect("Failed to clone TcpStream");
        ClientStream {
            stream_write:Mutex::new(stream),
            stream_read:RefCell::new(stream_read),
            addr
        }
    }
}

impl Client {
    pub fn new(stream: TcpStream, addr: SocketAddr) -> Self  {
        Client {
            stream: Arc::new(ClientStream::new(stream, addr)),
            data: RefCell::new(ClientData::new()),
        }
    }
}