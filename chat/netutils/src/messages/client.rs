use serde;

//messages sent from client to server

pub enum Message {
    OnRegisterUser,
    OnSent,
}

#[derive(serde::Serialize, serde::Deserialize, Debug)]
pub struct MsgOnRegisterUser<'a> {
    pub user: &'a str,
}

#[derive(serde::Serialize, serde::Deserialize, Debug)]
pub struct MsgOnSent<'a> {
    pub msg: &'a str,
}