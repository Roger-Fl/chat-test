use std::{
    sync::{mpsc::{self, Sender}, Arc},
};

use client_lib::{ClientState, ClientHandler, client::ClientStream};
use netutils::{message_stream::{self, MsgInfo}};
use netutils::messages;

pub enum MainThreadCode {
    NameAlreadyRegistered,
    RegistrationSucecssful,
    Disconnected,
}

pub struct ClientImpl {
    sender: mpsc::Sender<MainThreadCode>,
}

impl ClientImpl {
    pub fn new(sender: mpsc::Sender<MainThreadCode>) -> Self {
        ClientImpl {
            sender
        }
    }

    fn on_read(&self, client_state: &ClientState, stream: &ClientStream, msginfo: &MsgInfo) {
        match msginfo.code {
            x if msginfo.code == messages::server::Message::OnRegisterUser as u32 => {
                let msg = match msginfo.decode_data::<messages::server::MsgOnRegisterUser>() {
                    Ok(data) => data,
                    Err(e) => {
                        println!("{}", e);
                        return;
                    },
                };
    
                println!("{} has joined the server!", msg.user);
            },
            x if msginfo.code == messages::server::Message::OnAlreadyRegisteredUser as u32 => {
                let msg = match msginfo.decode_data::<messages::server::MsgAlreadyRegisteredUser>() {
                    Ok(data) => data,
                    Err(e) => {
                        println!("{}", e);
                        return;
                    },
                };
    
                println!("{} username already taken!", msg.user);
                self.sender.send(MainThreadCode::NameAlreadyRegistered).expect("Failed to send msg to main thread");
            },
            x if msginfo.code == messages::server::Message::OnRegistrationSuccess as u32 => {
                let msg = match msginfo.decode_data::<messages::server::MsgRegistrationSuccess>() {
                    Ok(data) => data,
                    Err(e) => {
                        println!("{}", e);
                        return;
                    },
                };
    
                println!("{} username registered!", msg.user);
                self.sender.send(MainThreadCode::RegistrationSucecssful).expect("Failed to send msg to main thread");
            },
            x if msginfo.code == messages::server::Message::OnDisconnect as u32 =>  {
                let msg = match msginfo.decode_data::<messages::server::MsgOnDisconnect>() {
                    Ok(data) => data,
                    Err(e) => {
                        println!("{}", e);
                        return;
                    },
                };
    
                println!("{} has disconnected from the server!", msg.user);
            },
            x if msginfo.code == messages::server::Message::OnSent as u32 => {
                let msg = match msginfo.decode_data::<messages::server::MsgOnSent>() {
                    Ok(data) => data,
                    Err(e) => {
                        println!("{}", e);
                        return;
                    },
                };
    
                println!("{} sent \"{}\"", msg.user, msg.msg);
            },
            code => {
                println!("unhandled message code: \"{}\"", code);  
            },
        }
    }
    
    fn on_disconnect(&self, client_state: &ClientState, stream: &ClientStream) {
        println!("Client disconnected from the server {}", stream.addr.to_string());
        self.sender.send(MainThreadCode::Disconnected).expect("Failed to send msg to main thread");
    }

    pub fn register_user(&self, client_state: &ClientState) -> String {
        let name: String = utils::io::read_val::<_, _, _> (
            "Enter name:",
            |_input: &str| format!("Expected string of length: [0, 15]").to_string(),
            Some(|val: &String| return val.len() <= 15 && val.len() > 0),
        );
    
        let msg = messages::client::MsgOnRegisterUser {
            user: name.as_str(),
        };
        let msg_encoded: Vec<u8> = message_stream::serialize_data(messages::client::Message::OnRegisterUser as u32, &msg).expect("Failed to serialze message");
        client_state.send(msg_encoded.as_slice()).expect("failed to send message");
    
        name
    }
}

pub fn client_handler_build(client_impl: Arc<ClientImpl>) -> ClientHandler {
    let client_impl0 = client_impl.clone();
    let client_impl1 = client_impl.clone();
    ClientHandler::new(Box::new(move |client_state: &ClientState, stream: &ClientStream, msginfo: &MsgInfo| client_impl0.on_read(client_state, stream, msginfo)), Box::new(move |client_state: &ClientState, stream: &ClientStream| client_impl1.on_disconnect(client_state, stream)))
}