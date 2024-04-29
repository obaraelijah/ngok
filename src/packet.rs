use serde::{Deserialize, Serialize};

#[derive(Debug, Serialize, Deserialize)]
pub enum Packet {
    Init,
    DataInit(String),
}

impl Packet {
    pub fn parse(bytes: &[u8]) -> Self {
        bincode::deserialize(bytes).unwrap()
    }
}