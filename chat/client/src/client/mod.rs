extern crate netutils;
extern crate utils;

mod client_impl;
use client_impl::{ClientImpl, MainThreadCode};

use client_lib::{Client};
use std::net::{TcpStream, SocketAddr, Ipv4Addr};
use std::time::Duration;
use std::sync::{Arc, mpsc};

use netutils::message_stream::{self};
use netutils::messages;
use netutils::logger::Logger;
use std::fs::File;

fn main() {
    let addr = SocketAddr::new(std::net::IpAddr::V4(Ipv4Addr::new(127, 0, 0, 1)), 7878);
    let stream = TcpStream::connect_timeout(&addr, Duration::from_secs(30)).expect("Failed to connect");

    let (sender, receiver) = mpsc::channel();
    let client_impl = Arc::new(ClientImpl::new(sender));
    let handler = client_impl::client_handler_build(client_impl.clone());
    let log_file = File::create("client_log.txt").expect("failed to create file client_log.txt");
    let logger = Arc::new(Logger::new(Some(Box::new(log_file))));
    let mut client = Client::new(stream, handler, logger);
    client.start();

    let mut name = client_impl.register_user(&client.state);

    loop {
        let res = receiver.recv();
        let code = match res {
            Ok(code) => code,
            Err(e) => {
                println!("{}", e);
                return;
            }
        };
    
        match code {
            MainThreadCode::NameAlreadyRegistered => {
                name = client_impl.register_user(&client.state);
            }
            MainThreadCode::RegistrationSucecssful => break,
            MainThreadCode::Disconnected => {
                println!("Main thread received disconnected code");
                return;
            }
        }
    }

    loop {
        let msg: String = utils::io::read_val::<_, _, _> (
            "Send (q for quit): ",
            |_input: &str| format!("Expected string").to_string(),
            None::<fn(&String) -> bool>,
        );

        if msg.to_lowercase() == "q".to_lowercase() {
            break;
        }

        let msg = messages::client::MsgOnSent {
            msg: msg.as_str(),
        };
        let msg_encoded: Vec<u8> = message_stream::serialize_data(messages::client::Message::OnSent as u32, &msg).expect("Failed to serialze message");
        client.state.send(msg_encoded.as_slice()).expect("failed to send message");
    }

    client.shutdown();
}
