use std::{
    net::{TcpStream},
    io::{ErrorKind, Read, Write},
    sync::{Arc},
};
use core::cell::RefCell;

use netutils::{thread_helper::{self, ThreadHelper}, message_stream::{self, MsgInfo}, logger::Logger};

mod client_error;
pub mod client;
use client_error::ClientError;
use client::{ClientData, ClientStream};

struct ClientThreads {
    read_thread: ThreadHelper,
}

impl ClientThreads {
    pub fn new(thread_state: Arc<thread_helper::ThreadState>, logger: Arc<Logger>) -> Self {
        ClientThreads {
            read_thread:ThreadHelper::new(thread_state.clone(), logger.clone()),
        }
    }

    pub fn start(&mut self, client: Arc<ClientState>) {
        //can have multiple const references
        let client_clone0 = client.clone();
        self.read_thread.start(String::from("Reader"), move || client_clone0.read_thread());
    }

    pub fn shutdown(&mut self) -> Result<(), thread_helper::ThreadError> {
        self.read_thread.shutdown()
    }

    pub fn wait_for_shutdown(&mut self) {
        self.read_thread.wait_for_shutdown();
    }
}

pub struct ClientState {
    thread_state: Arc<thread_helper::ThreadState>,
    handler: ClientHandler,
    pub stream: Arc<ClientStream>,
    data: RefCell<ClientData>,
    logger: Arc<Logger>,
}

unsafe impl Sync for ClientState {}
unsafe impl Send for ClientState {}

impl ClientState {
    pub fn new(thread_state: Arc<thread_helper::ThreadState>, stream: TcpStream, handler: ClientHandler, logger: Arc<Logger>) -> Self {
        stream.set_nonblocking(true).expect("Failed to put socket in nonblocking mode");
        ClientState {
            thread_state,
            handler,
            stream:Arc::new(ClientStream::new(stream)),
            data:RefCell::new(ClientData::new()),
            logger,
        }
    }

    pub fn send(&self, buffer: &[u8]) -> Result<(), std::io::Error> {
        self.stream.stream_write.lock().expect("failed to lock mutex").as_ref().expect("Invalid socket").write_all(buffer)?;
        Ok(())
    }

    pub fn shutdown(&self) -> Result<(), std::io::Error> {
        let res = self.thread_state.shutdown_start();
        match res {
            Ok(_) => {},
            Err(thread_helper::ThreadError::AlreadyShuttingDown) => {
                self.logger.log("Already shutting down...".as_bytes()).unwrap();
                return Ok(())
            },
            Err(e) => {self.logger.log(format!("{}", e).as_bytes()).unwrap();},
        }
        
        self.disconnect()
    }

    pub fn disconnect(&self) -> Result<(), std::io::Error> {
        //drop requires taking ownership of argument
        //need to drop both otherwise stream will not be shutdown (or shutting down write stream is sufficent)
        if let Some(stream) = self.stream.stream_write.lock().expect("Failed to lock mutex").take() {
            drop(stream);
        }
        (self.handler.on_disconnect)(&self, self.stream.as_ref());
        Ok(())
    }

    fn read_thread_cleanup(&self) {
        if let Some(stream) = self.stream.stream_read.take() {
            drop(stream);
        }
        self.data.borrow_mut().msg = Vec::new();
    }

    fn read_thread(&self) {
        self.logger.log("read_thread start".as_bytes()).unwrap();
        loop {
            if self.thread_state.is_shuttingdown() {
                self.logger.log("Threads are shutting down!!!!".as_bytes()).unwrap();
                break;
            }
            
            let res = self.read_server();
            if let Err(e) = res {
                self.logger.log(format!("{}", e).as_bytes()).unwrap();
            }
        }

        self.read_thread_cleanup();
        self.logger.log("read_thread done".as_bytes()).unwrap();
    }

    fn read_server(&self) -> Result<(), ClientError>  {
        const BUFF_SZ: usize = 4096;
        let mut buffer: [u8; BUFF_SZ] = [0; BUFF_SZ];

        match(self.read_helper(&mut buffer)) {
            Ok(_) => {},
            Err(ClientError::IoError(e)) => {
                let error_kind = e.kind();
                if error_kind != ErrorKind::WouldBlock && error_kind != ErrorKind::Interrupted {
                    self.logger.log(format!("Read error: {}, kind {}", e, error_kind).as_bytes()).unwrap();
                    self.shutdown()?;
                }
                return Ok(())
            },
            Err(ClientError::MsgError(e)) => {
                self.logger.log(format!("{}", e).as_bytes()).unwrap();
                if !self.thread_state.is_shuttingdown() {
                    self.data.borrow_mut().msg = Vec::new();
                }
                return Ok(())
            },
            Err(ClientError::ThreadError(e)) => {
                return Err(ClientError::ThreadError(e));
            },
        }

        Ok(())
    }

    fn read_helper(&self, buffer: &mut [u8]) -> Result<(), ClientError> {
        let read = self.stream.stream_read.borrow_mut().as_ref().expect("Invalid socket").read(buffer)?;

        let msg = &mut self.data.borrow_mut().msg;
        msg.extend_from_slice(&buffer[..read]);
        
        let mut msg_buffer = msg.as_slice();

        loop {
            let msgstream = match message_stream::parse_msgstream(msg_buffer) {
                Ok(Some(msginfo)) => msginfo,
                Ok(_) => {
                    let mut msg_rem = Vec::new();
                    msg_rem.extend_from_slice(msg_buffer);
                    *msg = msg_rem;
                    break;
                },
                Err(e) => { 
                    return Err(e.into()) 
                },
            };
    
            (self.handler.on_read)(&self, self.stream.as_ref(), &msgstream.msginfo);
            msg_buffer = msgstream.buffer_rem;
        }  

        Ok(())
    }
}

pub struct Client {
    thread_state: Arc<thread_helper::ThreadState>,
    threads: ClientThreads,
    pub state: Arc<ClientState>,
}

impl Client {
    pub fn new(stream: TcpStream, handler: ClientHandler, logger: Arc<Logger>) -> Self {
        let thread_state = Arc::new(thread_helper::ThreadState::new());
        Client {
            thread_state:thread_state.clone(),
            threads:ClientThreads::new(thread_state.clone(), logger.clone()),
            state:Arc::new(ClientState::new(thread_state.clone(), stream, handler, logger.clone()))
        }
    }

    pub fn start(&mut self) {
        self.threads.start(self.state.clone());
    }

    pub fn shutdown(&mut self) -> Result<(), std::io::Error> {
        self.state.shutdown()?;
        self.threads.wait_for_shutdown();

        Ok(())
    }
}

impl Drop for Client {
    fn drop(&mut self) {
        match self.shutdown() {
            Ok(_) => {},
            Err(e) => panic!("{}", e),
        }
    }
}

pub struct ClientHandler {
    on_read: Box<dyn Fn(&ClientState, &ClientStream, &MsgInfo)>,
    on_disconnect: Box<dyn Fn(&ClientState, &ClientStream)>,
}

impl ClientHandler {
    pub fn new(on_read: Box<dyn Fn(&ClientState, &ClientStream, &MsgInfo)>, on_disconnect: Box<dyn Fn(&ClientState, &ClientStream)>) -> Self {
        ClientHandler {
            on_read,
            on_disconnect
        }
    }
}

unsafe impl Sync for ClientHandler {}
unsafe impl Send for ClientHandler {}