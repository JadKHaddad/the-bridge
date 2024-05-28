#[derive(Debug)]
pub enum FramedReadError<E> {
    Io(E),
    Decode(bincode::error::DecodeError),
    ReadZero,
    BufferIsFull,
}

#[cfg(feature = "std")]
const _: () = {
    impl<E: std::fmt::Display> std::fmt::Display for FramedReadError<E> {
        fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
            match self {
                Self::Io(error) => write!(f, "IO error: {}", error),
                Self::Decode(error) => write!(f, "Decode error: {}", error),
                Self::ReadZero => write!(f, "Read zero bytes"),
                Self::BufferIsFull => write!(f, "Buffer is full"),
            }
        }
    }

    impl<E: std::error::Error> std::error::Error for FramedReadError<E> {}
};

#[cfg(feature = "defmt")]
const _: () = {
    impl<E: defmt::Format> defmt::Format for FramedReadError<E> {
        fn format(&self, f: defmt::Formatter) {
            match self {
                Self::Io(error) => defmt::write!(f, "IO error: {}", error),
                Self::Decode(_) => defmt::write!(f, "Decode error"),
                Self::ReadZero => defmt::write!(f, "Read zero bytes"),
                Self::BufferIsFull => defmt::write!(f, "Buffer is full"),
            }
        }
    }
};
