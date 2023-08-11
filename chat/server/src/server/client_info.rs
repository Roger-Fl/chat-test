#[derive(Debug, Clone)]
pub struct ClientInfo {
    pub name: String
}

impl ClientInfo {
    pub fn new(name: String) -> Self {
        ClientInfo {
            name
        }
    }
}