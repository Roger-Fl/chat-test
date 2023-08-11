extern crate netutils;
extern crate utils;

mod client_info;
mod server_impl;

use server_impl::{ServerImpl};

use server_lib::{Server};
use std::fs::File;
use std::net::{TcpListener};
use std::sync::{Arc};

use netutils::logger::Logger;

fn main() {
    let listener = TcpListener::bind("127.0.0.1:7878").expect("Failed to call bind");

    let server_impl = Arc::new(ServerImpl::new());
    let handler = server_impl::server_handler_build(server_impl.clone());
    let log_file = File::create("server_log.txt").expect("failed to create file server_log.txt");
    let logger = Arc::new(Logger::new(Some(Box::new(log_file))));
    let mut server = Server::new(listener, handler, logger);
    server.start();

    loop {
        let command: String = utils::io::read_val::<_, _, _> (
            "Enter command(q for quit):",
            |_input: &str| format!("Expected string of length: [0, 15]").to_string(),
            Some(|val: &String| return val.len() <= 15 && val.len() > 0),
        );
    
        if command.to_lowercase() == "q".to_lowercase() {
            break;
        }
    }
}
