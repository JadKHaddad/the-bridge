#[derive(Debug)]
pub enum EncodeError<E> {
    Io(E),
    Encode(bincode::error::EncodeError),
    BufferTooShort,
    MessageTooLarge,
}

#[cfg(feature = "std")]
const _: () = {
    impl<E: std::fmt::Display> std::fmt::Display for EncodeError<E> {
        fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
            match self {
                Self::Io(error) => write!(f, "IO error: {}", error),
                Self::Encode(error) => write!(f, "Encode error: {}", error),
                Self::BufferTooShort => write!(f, "Buffer is too short"),
                Self::MessageTooLarge => write!(f, "Message is too large"),
            }
        }
    }

    impl<E: std::error::Error> std::error::Error for EncodeError<E> {}
};

#[cfg(feature = "defmt")]
const _: () = {
    impl<E: defmt::Format> defmt::Format for EncodeError<E> {
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
