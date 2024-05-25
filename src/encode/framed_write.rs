use super::{async_write::AsyncWrite, error::EncodeError};
use core::marker::PhantomData;

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

        let packet_size = message_size as u32 + 4;

        // Write packet size
        buf[0] = (packet_size >> 24) as u8;
        buf[1] = (packet_size >> 16) as u8;
        buf[2] = (packet_size >> 8) as u8;
        buf[3] = packet_size as u8;

        self.writer
            .write_all(&buf[..packet_size as usize])
            .await
            .map_err(EncodeError::Io)?;

        #[cfg(feature = "tracing")]
        {
            let message_buf = &buf[..message_size];
            tracing::trace!(%packet_size, %message_size, ?message_buf, "Message encoded");
        }

        #[cfg(feature = "log")]
        {
            let message_buf = &buf[..message_size];
            log::trace!(
                "Message encoded. Packet size: {}, message size: {}, message: {:?}",
                packet_size,
                message_size,
                message_buf
            );
        }

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
        ) -> impl Sink<M, Error = EncodeError<W::Error>> + Captures<&'a Self> {
            futures::sink::unfold(self, |this, item: M| async move {
                this.write_frame(&item).await?;

                Ok::<_, EncodeError<W::Error>>(this)
            })
        }

        pub fn into_sink(self) -> impl Sink<M, Error = EncodeError<W::Error>> + Captures<&'a Self> {
            futures::sink::unfold(self, |mut this, item: M| async move {
                this.write_frame(&item).await?;

                Ok::<_, EncodeError<W::Error>>(this)
            })
        }
    }
};
