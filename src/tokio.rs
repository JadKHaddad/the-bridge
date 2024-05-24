use crate::{
    decode::{async_read::AsyncRead, error::DecodeError},
    encode::{async_write::AsyncWrite, error::EncodeError},
};
use core::{future::Future, marker::PhantomData};
use futures::io::Error as IoError;
use tokio::io::{
    AsyncRead as TokioAsyncRead, AsyncReadExt, AsyncWrite as TokioAsyncWrite, AsyncWriteExt,
};
use tokio_util::{
    bytes::{Buf, BufMut, BytesMut},
    codec::{Decoder, Encoder},
};

pub struct Compat<T>(T);

impl<T> Compat<T> {
    pub fn new(inner: T) -> Self {
        Self(inner)
    }

    pub fn get_ref(&self) -> &T {
        &self.0
    }

    pub fn get_mut(&mut self) -> &mut T {
        &mut self.0
    }

    pub fn into_inner(self) -> T {
        self.0
    }
}

impl<R> AsyncRead for Compat<R>
where
    R: TokioAsyncRead + Unpin,
{
    type Error = IoError;

    fn read<'a>(
        &'a mut self,
        buf: &'a mut [u8],
    ) -> impl Future<Output = Result<usize, Self::Error>> {
        self.0.read(buf)
    }
}

impl<W> AsyncWrite for Compat<W>
where
    W: TokioAsyncWrite + Unpin,
{
    type Error = IoError;

    fn write_all<'a>(&'a mut self, buf: &'a [u8]) -> impl Future<Output = Result<(), Self::Error>> {
        self.0.write_all(buf)
    }
}

pub struct Codec<M> {
    _phantom: PhantomData<M>,
}

impl<M> Default for Codec<M> {
    fn default() -> Self {
        Self {
            _phantom: PhantomData,
        }
    }
}

/// Implement [`From`] [`IoError`] for [`DecodeError`] to be be able to implement [`Decoder`] for [`Codec`]
impl<IoError> From<IoError> for DecodeError<IoError> {
    fn from(err: IoError) -> Self {
        DecodeError::Io(err)
    }
}

impl<M> Decoder for Codec<M>
where
    M: bincode::Decode,
{
    type Item = M;
    type Error = DecodeError<IoError>;

    fn decode(&mut self, src: &mut BytesMut) -> Result<Option<Self::Item>, Self::Error> {
        if src.len() < 4 {
            #[cfg(feature = "tracing")]
            {
                tracing::trace!(
                    source_length = src.len(),
                    "Not enough bytes to read packet size"
                );
            }

            return Ok(None);
        }

        let packet_size = u32::from_be_bytes([src[0], src[1], src[2], src[3]]) as usize;

        #[cfg(feature = "tracing")]
        {
            tracing::trace!(%packet_size, "Checking if enough bytes are available");
        }

        if src.len() < packet_size + 4 {
            #[cfg(feature = "tracing")]
            {
                let remaining = packet_size + 4 - src.len();
                tracing::trace!(%remaining, "Not enough bytes to decode the packet. Breaking");
            }

            src.reserve(packet_size + 4 - src.len());

            return Ok(None);
        }

        let message_buf = &src[4..packet_size + 4];

        #[cfg(feature = "tracing")]
        {
            let packet_buf = &src[..packet_size + 4];
            tracing::trace!(?packet_buf, ?message_buf, "Decoding message");
        }

        let message = bincode::decode_from_slice(message_buf, bincode::config::standard())
            .map_err(DecodeError::Decode)?;

        src.advance(packet_size + 4);

        Ok(Some(message.0))
    }
}

/// Implement [`From`] [`IoError`] for [`EncodeError`] to be be able to implement [`Encoder`] for [`Codec`]
impl<IoError> From<IoError> for EncodeError<IoError> {
    fn from(err: IoError) -> Self {
        EncodeError::Io(err)
    }
}

impl<M> Encoder<&M> for Codec<M>
where
    M: bincode::Encode,
{
    type Error = EncodeError<IoError>;

    fn encode(&mut self, item: &M, dst: &mut BytesMut) -> Result<(), Self::Error> {
        let message = bincode::encode_to_vec(item, bincode::config::standard())
            .map_err(EncodeError::Encode)?;

        let message_size = message.len();
        let packet_size = message_size + 4;

        if message_size > u32::MAX as usize {
            return Err(EncodeError::MessageTooLarge);
        }

        dst.reserve(packet_size);
        dst.put_u32(packet_size as u32);
        dst.put_slice(&message);

        #[cfg(feature = "tracing")]
        {
            tracing::trace!(%packet_size, %message_size, message_buf=?message, "Message encoded");
        }

        Ok(())
    }
}
