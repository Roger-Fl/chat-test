use std::{
    sync::{mpsc::{self, Sender}, Arc, Mutex},
};
use std::net::{SocketAddr};

use super::client_info;
use server_lib::{ServerHandler, ServerState, client::ClientStream};
use netutils::{message_stream::{self, MsgInfo}};
use netutils::messages;
use std::collections::HashMap;

pub struct ServerImpl {
    clients: Mutex<HashMap<SocketAddr, client_info::ClientInfo>>
}

impl ServerImpl {
    pub fn new() -> Self {
        ServerImpl {
            clients: Mutex::new(HashMap::new()),
        }
    }

    fn allow_connect(&self, server_state: &ServerState, stream: &ClientStream) -> bool {
        true
    }
    
    fn on_connect(&self, server_state: &ServerState, stream: &ClientStream) {
        println!("Client {} connected to the server", stream.addr);
        // let msg = messages::MsgOnConnect {
        //     user: "Test",
        // };
        // let msg_encoded = message_stream::serialize_data(messages::Message::OnConnect as u32, &msg).expect("Failed to serialze message");
        // server_state.send_all_except_s(msg_encoded.as_slice(), stream.addr).expect("failed to send message");
    }
    
    fn on_disconnect(&self, server_state: &ServerState, stream: &ClientStream) {
        let cdata = match self.clients.lock().expect("Failed to lock mutex").remove(&stream.addr) {
            Some(data) => data,
            None => {
                println!("Error: client {} not currently in registered clients", stream.addr);
                return;
            }
        };
        println!("Client name: {}, addr: {} disconnected from the server", cdata.name, stream.addr.to_string());
    
        let msg = messages::server::MsgOnDisconnect {
            user: cdata.name.as_str(),
        };
        let msg_encoded = message_stream::serialize_data(messages::server::Message::OnDisconnect as u32, &msg).expect("Failed to serialze message");
        server_state.send_all_except_s(msg_encoded.as_slice(), stream.addr).expect("failed to send message");
    }
    
    fn on_read(&self, server_state: &ServerState, stream: &ClientStream, msginfo: &MsgInfo) {
        match msginfo.code {
            x if msginfo.code == messages::client::Message::OnRegisterUser as u32 => {
                let msg = match msginfo.decode_data::<messages::client::MsgOnRegisterUser>() {
                    Ok(data) => data,
                    Err(e) => {
                        println!("{}", e);
                        return;
                    },
                };
    
                let mut lock_guard = self.clients.lock().expect("Failed to lock mutex");
                if lock_guard.iter().any(|(k, v)| v.name == msg.user) {
                    drop(lock_guard);
    
                    println!("{} is already registered!", msg.user);
    
                    let msg = messages::server::MsgAlreadyRegisteredUser {
                        user: msg.user
                    };
                    let msg_encoded = message_stream::serialize_data(messages::server::Message::OnAlreadyRegisteredUser as u32, &msg).expect("Failed to serialze message");
                    server_state.send(&stream, msg_encoded.as_slice()).expect("failed to send message");
                }
                else {
                    lock_guard.insert(stream.addr, client_info::ClientInfo::new(msg.user.to_string()));
                    drop(lock_guard);
    
                    println!("{} has registered!", msg.user);
    
                    let msg = messages::server::MsgRegistrationSuccess {
                        user: msg.user
                    };
                    let msg_encoded = message_stream::serialize_data(messages::server::Message::OnRegistrationSuccess as u32, &msg).expect("Failed to serialze message");
                    server_state.send(&stream, msg_encoded.as_slice()).expect("failed to send message");
                }
            },
            x if msginfo.code == messages::client::Message::OnSent as u32 => {
                let msg = match msginfo.decode_data::<messages::client::MsgOnSent>() {
                    Ok(data) => data,
                    Err(e) => {
                        println!("{}", e);
                        return;
                    },
                };
    
                let cdata = match self.clients.lock().expect("Failed to lock mutex").get(&stream.addr) {
                    Some(data) => data.clone(),
                    None => return,
                };
    
                let msg = messages::server::MsgOnSent {
                    user: cdata.name.as_str(),
                    msg: msg.msg,
                };
                let msg_encoded: Vec<u8> = message_stream::serialize_data(messages::server::Message::OnSent as u32, &msg).expect("Failed to serialze message");
                server_state.send_all_except_s(msg_encoded.as_slice(), stream.addr).expect("failed to send message");
    
                println!("{} sent \"{}\"", msg.user, msg.msg);
            },
            code => {
                println!("Unknown or nhandled message code: \"{}\"", code);  
            },
        }
    }
}

pub fn server_handler_build(server_impl: Arc<ServerImpl>) -> ServerHandler {
    let server_impl0 = server_impl.clone();
    let server_impl1 = server_impl.clone();
    let server_impl2 = server_impl.clone();
    let server_impl3 = server_impl.clone();

    ServerHandler::new(
        Box::new(move |server_state: &ServerState, stream: &ClientStream| server_impl0.allow_connect(server_state, stream)),
        Box::new(move |server_state: &ServerState, stream: &ClientStream| server_impl1.on_connect(server_state, stream)),
        Box::new(move |server_state: &ServerState, stream: &ClientStream| server_impl2.on_disconnect(server_state, stream)),
        Box::new(move |server_state: &ServerState, stream: &ClientStream, msginfo: &MsgInfo| server_impl3.on_read(server_state, stream, msginfo)), 
    )
}