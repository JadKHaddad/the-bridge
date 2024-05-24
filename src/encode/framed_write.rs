use super::{async_write::AsyncWrite, error::EncodeError};
use core::marker::PhantomData;

pub struct FramedWrite<'a, W, M> {
    writer: W,
    buf: &'a mut [u8],
    _phantom: PhantomData<M>,
}

impl<'a, W: AsyncWrite, M: bincode::Encode> FramedWrite<'a, W, M> {
    pub fn new(writer: W, buf: &'a mut [u8]) -> Self {
        Self {
            writer,
            buf,
            _phantom: PhantomData,
        }
    }

    pub fn into_inner(self) -> W {
        self.writer
    }

    pub async fn write_frame(&mut self, message: &M) -> Result<(), EncodeError<W::Error>> {
        let buf = self.buf.as_mut();

        if buf.len() < 4 {
            return Err(EncodeError::BufferTooShort);
        }

        // Encode message starting from the 5th byte, leaving the first 4 bytes for the packet size
        let message_size =
            bincode::encode_into_slice(message, &mut buf[4..], bincode::config::standard())
                .map_err(EncodeError::Encode)?;

        if message_size > u32::MAX as usize {
            return Err(EncodeError::MessageTooLarge);
        }

        let packet_size = message_size as u32;

        // Write packet size
        buf[0] = (packet_size >> 24) as u8;
        buf[1] = (packet_size >> 16) as u8;
        buf[2] = (packet_size >> 8) as u8;
        buf[3] = packet_size as u8;

        self.writer
            .write_all(&buf[..packet_size as usize + 4])
            .await
            .map_err(EncodeError::Io)?;

        #[cfg(feature = "tracing")]
        {
            let message_buf = &buf[4..message_size + 4];
            tracing::trace!(%packet_size, %message_size, ?message_buf, "Message encoded");
        }

        Ok(())
    }
}
