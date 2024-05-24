#[derive(Debug)]
pub enum EncodeError<E> {
    Io(E),
    Encode(bincode::error::EncodeError),
    BufferTooShort,
    MessageTooLarge,
}
