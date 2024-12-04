use serde::{Deserialize, Serialize};


#[derive(Debug, Serialize, Deserialize)]
pub struct Device {
    pub id: i32,
    // pub name: String,
    pub pin: i32,
    pub state: bool,
    // pub io: IO,
    // pub icon: u32
}

#[derive(Serialize, Deserialize)]
pub enum IO {
    PinInput,
    PinOutput
}