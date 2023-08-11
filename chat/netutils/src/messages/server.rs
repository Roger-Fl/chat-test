use serde;

//messages sent from server to client

pub enum Message {
    OnConnect,
    OnDisconnect,
    OnRegisterUser,
    OnAlreadyRegisteredUser,
    OnRegistrationSuccess,
    OnSent,
}

#[derive(serde::Serialize, serde::Deserialize, Debug)]
pub struct MsgOnConnect {
}

#[derive(serde::Serialize, serde::Deserialize, Debug)]
pub struct MsgOnRegisterUser<'a> {
    pub user: &'a str,
}

#[derive(serde::Serialize, serde::Deserialize, Debug)]
pub struct MsgAlreadyRegisteredUser<'a> {
    pub user: &'a str,
}

#[derive(serde::Serialize, serde::Deserialize, Debug)]
pub struct MsgRegistrationSuccess<'a> {
    pub user: &'a str,
}

#[derive(serde::Serialize, serde::Deserialize, Debug)]
pub struct MsgOnDisconnect<'a> {
    pub user: &'a str,
}

#[derive(serde::Serialize, serde::Deserialize, Debug)]
pub struct MsgOnSent<'a, 'b> {
    pub user: &'a str,
    pub msg: &'b str,
}