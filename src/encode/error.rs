#[derive(Debug)]
pub enum FramedWriteError<E> {
    Io(E),
    Encode(bincode::error::EncodeError),
    BufferTooShort,
    MessageTooLarge,
}

#[cfg(feature = "std")]
const _: () = {
    impl<E: std::fmt::Display> std::fmt::Display for FramedWriteError<E> {
        fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
            match self {
                Self::Io(error) => write!(f, "IO error: {}", error),
                Self::Encode(error) => write!(f, "Encode error: {}", error),
                Self::BufferTooShort => write!(f, "Buffer is too short"),
                Self::MessageTooLarge => write!(f, "Message is too large"),
            }
        }
    }

    impl<E: std::error::Error> std::error::Error for FramedWriteError<E> {}
};

#[cfg(feature = "defmt")]
const _: () = {
    impl<E: defmt::Format> defmt::Format for FramedWriteError<E> {
        fn format(&self, f: defmt::Formatter) {
            match self {
                Self::Io(error) => defmt::write!(f, "IO error: {}", error),
                Self::Encode(_) => defmt::write!(f, "Encode error"),
                Self::BufferTooShort => defmt::write!(f, "Buffer is too short"),
                Self::MessageTooLarge => defmt::write!(f, "Message is too large"),
            }
        }
    }
};
