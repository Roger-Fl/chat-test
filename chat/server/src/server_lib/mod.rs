use std::{
    net::{TcpListener, Shutdown, SocketAddr},
    io::{ErrorKind, Read, Write},
    sync::{Arc, RwLock},
    collections::{HashSet, HashMap},
    time,
    error::Error,
    thread,
};

use netutils::{thread_helper::{self, ThreadHelper}, message_stream::{self, MsgInfo, MsgError}, logger::Logger};

pub mod client;
mod server_error;
use server_error::ServerError;
use client::{Client, ClientStream};

struct ServerThreads {
    listener_thread: ThreadHelper,
    read_thread: ThreadHelper,
}

impl ServerThreads {
    pub fn new(thread_state: Arc<thread_helper::ThreadState>, logger: Arc<Logger>) -> Self {
        ServerThreads {
            listener_thread:ThreadHelper::new(thread_state.clone(), logger.clone()),
            read_thread:ThreadHelper::new(thread_state.clone(), logger.clone()),
        }
    }

    pub fn start(&mut self, server: Arc<ServerState>) {
        //can have multiple immutable references
        let server_clone0 = server.clone();
        let server_clone1 = server.clone();
        self.read_thread.start(String::from("Reader"), move || server_clone1.read_thread());
        self.listener_thread.start(String::from("Listener"), move || server_clone0.listen_thread());
    }

    pub fn shutdown(&mut self) -> Result<(), thread_helper::ThreadError> {
        self.listener_thread.shutdown()?;
        self.read_thread.shutdown()?;
        Ok(())
    }

    pub fn wait_for_shutdown(&mut self) {
        self.listener_thread.wait_for_shutdown();
        self.read_thread.wait_for_shutdown();
    }
}

pub struct ServerState {
    thread_state: Arc<thread_helper::ThreadState>,
    listener: TcpListener,
    handler: ServerHandler,
    pub clients_stream: RwLock<HashMap<SocketAddr, Arc<Client>>>, //mutex gets around const reference
    logger: Arc<Logger>,
}

impl ServerState {
    pub fn new(thread_state: Arc<thread_helper::ThreadState>, listener: TcpListener, handler: ServerHandler,logger: Arc<Logger>) -> Self {
        listener.set_nonblocking(true).expect("Failed to set TcpListener to nonblocking");
        ServerState {
            thread_state,
            listener,
            clients_stream:RwLock::new(HashMap::new()),
            handler,
            logger,
        }
    }

    pub fn disconnect(&self) -> Result<(), std::io::Error> {
        let to_remove = self.clients_stream.read().expect("Failed to lock mutex").iter().map(|(_, v)| v.clone()).collect();
        self.handle_disconnected_clients(&to_remove)?;
        Ok(())
    }

    pub fn shutdown(&self) -> Result<(), std::io::Error> {
        let res = self.thread_state.shutdown_start();
        match res {
            Ok(_) => {},
            Err(thread_helper::ThreadError::AlreadyShuttingDown) => {self.logger.log("Already shutting down!!!".as_bytes()).unwrap();},
            Err(e) => {self.logger.log(format!("{}", e).as_bytes()).unwrap();},
        }    

        self.disconnect()
    }

    pub fn send(&self, stream: &ClientStream, buffer: &[u8]) -> Result<(), std::io::Error> {
        let mut lock_guard = stream.stream_write.lock().expect("Failed to lock mutex");
        let res = lock_guard.write_all(buffer);
        if let Err(e) = res {
            let client = self.clients_stream.write().expect("Failed to lock mutex").remove(&stream.addr).expect("Failed to unwrap client");
            lock_guard.shutdown(Shutdown::Both)?;
            drop(lock_guard);

            (self.handler.on_disconnect)(&self, stream);
        }
        Ok(())
    }
    pub fn send_all_it<It>(&self, buffer: &[u8], clients: It) -> Result<(), std::io::Error> 
    where
        It: IntoIterator<Item=Arc<Client>>
    {
        let mut to_remove: Vec<Arc<Client>> = vec![];
        for client in clients {
            let res = client.as_ref().stream.stream_write.lock().expect("Failed to lock mutex").write_all(buffer);
            if let Err(e) = res {
                to_remove.push(client.clone());
            }
        }

        self.handle_disconnected_clients(&to_remove)?;
        Ok(())
    }
    // pub fn send_all_it2<It, T>(&self, buffer: &[u8], clients: It) -> Result<(), std::io::Error> 
    // where
    //     T: AsRef<Arc<Client>>,
    //     It: IntoIterator<Item=T>
    // {
    //     let mut to_remove: Vec<SocketAddr> = vec![];
    //     for client in clients {
    //         let res = client.as_ref().stream.stream_write.lock().expect("Failed to lock mutex").write_all(buffer);
    //         if let Err(e) = res {
    //             to_remove.push(client.as_ref().stream.addr);
    //         }
    //     }

    //     self.handle_disconnected_clients(to_remove)?;
    //     Ok(())
    // }
    pub fn send_all(&self, buffer: &[u8]) -> Result<(), std::io::Error> {
        let clients: Vec<Arc<Client>> = self.clients_stream.read().expect("Failed to lock mutex")
            .iter()
            .map(|(_, client)| client.clone())
            .collect();

        self.send_all_it(&buffer, clients.into_iter())?;
        Ok(())
    }
    pub fn send_all_except<I>(&self, buffer: &[u8], excluded: &HashSet<SocketAddr>) -> Result<(), std::io::Error> {
        let clients: Vec<Arc<Client>> = self.clients_stream.read().expect("Failed to lock mutex")
            .iter()
            .filter(|(addr, _)| !excluded.contains(&addr))
            .map(|(_, client)| client.clone())
            .collect();

        self.send_all_it(&buffer, clients.into_iter())?;
        Ok(())
    }
    pub fn send_all_except_s(&self, buffer: &[u8], excluded: SocketAddr) -> Result<(), std::io::Error> {
        let clients: Vec<Arc<Client>> = self.clients_stream.read().expect("Failed to lock mutex")
            .iter()
            .filter(|(addr, _)| excluded != **addr)
            .map(|(_, client)| client.clone())
            .collect();

        self.send_all_it(&buffer, clients.into_iter())?;
        Ok(())
    }

    fn listen_thread(&self) {
        self.logger.log("listen_thread start".as_bytes()).unwrap();
        loop {
            if self.thread_state.is_shuttingdown() {
                self.logger.log("Threads are shutting down!!!!".as_bytes()).unwrap();
                break;
            }

            let res = self.accept_clients();
            match res {
                Ok(_) => {},
                Err(e) => println!("{}", e),
            }
        }
        self.logger.log("listen_thread done".as_bytes()).unwrap();
    }

    fn read_thread(&self) {
        self.logger.log("read_thread start".as_bytes()).unwrap();
        loop {
            if self.thread_state.is_shuttingdown() {
                self.logger.log("Threads are shutting down!!!!".as_bytes()).unwrap();
                break;
            }

            //cannot clone hashmap unless both key and value implement clone trait
            let to_remove = match self.read_clients() {
                Ok(to_remove) => to_remove,
                Err(e) => {
                    println!("{}", e);
                    continue;
                }
            };
            let res = self.handle_disconnected_clients(&to_remove);
            if let Err(e) = res {
                self.logger.log(format!("{}", e).as_bytes()).unwrap();
            }
        }
        self.logger.log("read_thread done".as_bytes()).unwrap();
    }

    fn read_clients(&self) -> Result<Vec<Arc<Client>>, std::io::Error> {
        const BUFF_SZ: usize = 4096;
        let mut buffer: [u8; BUFF_SZ] = [0; BUFF_SZ];
        let mut to_remove = Vec::new();
        let clients: Vec<Arc<Client>> = self.clients_stream.read().expect("Failed to lock mutex")
            .iter()
            .map(|(_, client)| client.clone())
            .collect();
        
        for client in clients.iter() {   
            if self.thread_state.is_shuttingdown() {
                break;
            }

            let res = self.read_client_helper(client, &mut buffer);
            match res {
                Ok(_) => {},
                Err(ServerError::IoError(e)) => {
                    let error_kind = e.kind();
                    if error_kind != ErrorKind::WouldBlock && error_kind != ErrorKind::Interrupted {
                        self.logger.log(format!("Read error: {}, kind {}", e, error_kind).as_bytes())?;
                        to_remove.push(client.clone());
                    }
                    continue;
                }
                Err(ServerError::ReadError(s)) => {
                    self.logger.log(format!("{}", s).as_bytes())?;
                    to_remove.push(client.clone());
                }
                Err(ServerError::MsgError(e)) => {
                    self.logger.log(format!("{}", e).as_bytes())?;
                    client.data.borrow_mut().msg = Vec::new();
                }
                Err(e) => {
                    self.logger.log(format!("{}", e).as_bytes())?;
                    to_remove.push(client.clone());
                }
            }
        }
        Ok(to_remove)
    }

    fn read_client_helper(&self, client: &Arc<Client>, buffer: &mut [u8]) -> Result<(), ServerError> {
        let read = client.stream.stream_read.borrow_mut().read(buffer)?;

        if read == 0 {
            return Err(ServerError::ReadError(format!("Read error: {} bytes read", read)))
        }
        
        let client_data = &mut client.data.borrow_mut();
        let msg = &mut client_data.msg;
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
    
            (self.handler.on_read)(&self, client.stream.as_ref(), &msgstream.msginfo);
            msg_buffer = msgstream.buffer_rem;
        }  
        Ok(())
    }

    fn accept_clients(&self) -> Result<(), ServerError>  {
        let strm_res = self.listener.accept();

        match strm_res {
            Ok((stream, addr)) => {
                let client = Arc::new(Client::new(stream, addr));
                let stream = client.stream.clone();
                if (self.handler.allow_connect)(&self, stream.as_ref()) {
                    self.clients_stream.write().expect("Failed to lock mutex").insert(addr, client);
                    (self.handler.on_connect)(&self, stream.as_ref());
                }
            }
            Err(ref e) if e.kind() == ErrorKind::WouldBlock => {
                // wait until network socket is ready, typically implemented
                // via platform-specific APIs such as epoll or IOCP
                thread::sleep(time::Duration::from_millis(5));
            }
            Err(e) => {
                return Err(e.into());
            }
        }

        Ok(())
    }
    // fn handle_disconnected_clients<I>(&self, to_remove: I)
    // where
    //     I: Iterator<SocketAddr>
    // {
    fn handle_disconnected_clients(&self, to_remove: &Vec<Arc<Client>>) -> Result<(), std::io::Error> {
        if to_remove.len() > 0 {
            let mut lock_guard = self.clients_stream.write().expect("Failed to lock mutex");
            for client in to_remove.iter() {
                lock_guard.remove(&client.stream.addr).expect("Failed to unwrap client");
            }
            drop(lock_guard);

            for client in to_remove.iter() {
                client.stream.stream_write.lock().expect("Failed to lock mutex").shutdown(Shutdown::Both)?; //cannot propogate error in closure as foreach doesnt return result
                (self.handler.on_disconnect)(&self, client.stream.as_ref());
            }

            // to_remove.iter().for_each(
            //     |addr| 
            // {
            //     let client = lock_guard.remove(addr).expect("Failed to unwrap client");
            //     client.stream.stream_write.lock().expect("Failed to lock mutex").shutdown(Shutdown::Both)?; //cannot propogate error in closure as foreach doesnt return result
            //     (self.handler.on_disconnect)(&self, client.stream.as_ref());
            //     ()
            // });
        }    
        Ok(())
    }
}

pub struct Server {
    thread_state: Arc<thread_helper::ThreadState>,
    threads: ServerThreads,
    pub state: Arc<ServerState>,
}

impl Server {
    pub fn new(listener: TcpListener, handler: ServerHandler, logger: Arc<Logger>) -> Self {
        let thread_state = Arc::new(thread_helper::ThreadState::new());
        Server {
            thread_state:thread_state.clone(),
            threads:ServerThreads::new(thread_state.clone(), logger.clone()),
            state:Arc::new(ServerState::new(thread_state.clone(), listener, handler, logger.clone())),
        }
    }

    pub fn start(&mut self) {
        self.threads.start(self.state.clone());
    }

    pub fn shutdown(&mut self) -> Result<(), Box<dyn Error>> {
        self.state.shutdown()?;
        self.threads.wait_for_shutdown();
        Ok(())
    }
}

impl Drop for Server {
    fn drop(&mut self) {
        match self.shutdown() {
            Ok(_) => {},
            Err(e) => panic!("{}", e),
        }
    }
}


pub struct ServerHandler {
    allow_connect: Box<dyn Fn(&ServerState, &ClientStream)->bool>,
    on_connect: Box<dyn Fn(&ServerState, &ClientStream)>,
    on_disconnect: Box<dyn Fn(&ServerState, &ClientStream)>,
    on_read: Box<dyn Fn(&ServerState, &ClientStream, &MsgInfo)>,
}

impl ServerHandler {
    pub fn new(allow_connect: Box<dyn Fn(&ServerState, &ClientStream)->bool>, on_connect: Box<dyn Fn(&ServerState, &ClientStream)>, on_disconnect: Box<dyn Fn(&ServerState, &ClientStream)>, on_read: Box<dyn Fn(&ServerState, &ClientStream, &MsgInfo)>) -> Self {
        ServerHandler {
            allow_connect,
            on_connect,
            on_disconnect,
            on_read,
        }
    }
}

unsafe impl Sync for ServerHandler {}
unsafe impl Send for ServerHandler {}