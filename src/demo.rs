#[derive(Debug, Clone, bincode::Encode, bincode::Decode, PartialEq)]
pub enum DemoMessage {
    Ping(u32),
    Pong(u32),
    Measurement(i64),
}
