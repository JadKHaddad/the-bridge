use super::{async_write::AsyncWrite, error::FramedWriteError};
use core::marker::PhantomData;

// TODO: impl sink for FramedWrite
pub struct FramedWrite<'a, W, M> {
    writer: W,
    buf: &'a mut [u8],
    _phantom: PhantomData<M>,
}

impl<'a, W, M> AsRef<W> for FramedWrite<'a, W, M> {
    fn as_ref(&self) -> &W {
        &self.writer
    }
}

impl<'a, W, M> AsMut<W> for FramedWrite<'a, W, M> {
    fn as_mut(&mut self) -> &mut W {
        &mut self.writer
    }
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

    pub async fn write_frame(&mut self, message: &M) -> Result<(), FramedWriteError<W::Error>> {
        let buf = self.buf.as_mut();

        if buf.len() < 4 {
            return Err(FramedWriteError::BufferTooShort);
        }

        // Encode message starting from the 5th byte, leaving the first 4 bytes for the packet size
        let message_size =
            bincode::encode_into_slice(message, &mut buf[4..], bincode::config::standard())
                .map_err(FramedWriteError::Encode)?;

        if message_size > u32::MAX as usize {
            return Err(FramedWriteError::MessageTooLarge);
        }

        let packet_size = message_size as u32 + 4;

        // Write packet size
        buf[0] = (packet_size >> 24) as u8;
        buf[1] = (packet_size >> 16) as u8;
        buf[2] = (packet_size >> 8) as u8;
        buf[3] = packet_size as u8;

        self.writer
            .write_all(&buf[..packet_size as usize])
            .await
            .map_err(FramedWriteError::Io)?;

        #[cfg(any(feature = "tracing", feature = "log", feature = "defmt"))]
        let message_buf = &buf[..message_size];

        #[cfg(feature = "tracing")]
        tracing::trace!(%packet_size, %message_size, ?message_buf, "Message encoded");

        #[cfg(feature = "log")]
        log::info!(
            "Message encoded. packet_size: {}, message_size: {}, message_buf: {:?}",
            packet_size,
            message_size,
            message_buf
        );

        #[cfg(feature = "defmt")]
        defmt::info!(
            "Message encoded. packet_size: {}, message_size: {}, message_buf: {:?}",
            packet_size,
            message_size,
            message_buf
        );

        Ok(())
    }
}

#[cfg(feature = "futures")]
const _: () = {
    use crate::captures::Captures;
    use futures::Sink;

    impl<'a, W: AsyncWrite, M: bincode::Encode> FramedWrite<'a, W, M> {
        pub fn sink(
            &'a mut self,
        ) -> impl Sink<M, Error = FramedWriteError<W::Error>> + Captures<&'a Self> {
            futures::sink::unfold(self, |this, item: M| async move {
                this.write_frame(&item).await?;

                Ok::<_, FramedWriteError<W::Error>>(this)
            })
        }

        pub fn into_sink(
            self,
        ) -> impl Sink<M, Error = FramedWriteError<W::Error>> + Captures<&'a Self> {
            futures::sink::unfold(self, |mut this, item: M| async move {
                this.write_frame(&item).await?;

                Ok::<_, FramedWriteError<W::Error>>(this)
            })
        }
    }
};
