#[derive(Debug, Clone, bincode::Encode, bincode::Decode, PartialEq)]
#[cfg_attr(feature = "defmt", derive(defmt::Format))]
pub enum DemoMessage {
    Ping(u32),
    Pong(u32),
    Measurement(i64),
}
