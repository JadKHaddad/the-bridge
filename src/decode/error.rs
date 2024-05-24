#[derive(Debug)]
pub enum DecodeError<E> {
    Io(E),
    Decode(bincode::error::DecodeError),
    ReadZero,
    BufferIsFull,
}
