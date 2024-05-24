use futures_core::Stream;

use super::{async_read::AsyncRead, error::DecodeError};
use core::marker::PhantomData;

pub struct FramedRead<'a, R, M> {
    reader: R,
    buf: &'a mut [u8],
    cursor: usize,
    has_errored: bool,
    _phantom: PhantomData<M>,
}

impl<'a, R: AsyncRead, M: bincode::Decode> FramedRead<'a, R, M> {
    pub fn new(reader: R, buf: &'a mut [u8]) -> Self {
        Self {
            reader,
            buf,
            cursor: 0,
            has_errored: false,
            _phantom: PhantomData,
        }
    }

    pub fn into_inner(self) -> R {
        self.reader
    }

    async fn fill_buf(&mut self) -> Result<(), DecodeError<R::Error>> {
        let buf = self.buf.as_mut();
        let read = self
            .reader
            // Cursor will never be > buf.len()
            .read(&mut buf[self.cursor..])
            .await
            .map_err(DecodeError::Io)?;

        #[cfg(feature = "tracing")]
        {
            tracing::trace!(%read);
        }

        if read == 0 {
            if self.cursor == buf.len() {
                // The packet is too big
                return Err(DecodeError::BufferIsFull);
            }

            // Got EOF
            return Err(DecodeError::ReadZero);
        }

        self.cursor += read;

        #[cfg(feature = "tracing")]
        {
            let buffered = &self.buf[..self.cursor];
            tracing::trace!(cursor=%self.cursor, ?buffered, "Incremented cursor");
        }

        Ok(())
    }

    pub async fn read_frame(&mut self) -> Result<M, DecodeError<R::Error>> {
        loop {
            if self.cursor >= 4 {
                let packet_size =
                    u32::from_be_bytes([self.buf[0], self.buf[1], self.buf[2], self.buf[3]])
                        as usize;

                #[cfg(feature = "tracing")]
                {
                    let buffered = &self.buf[..self.cursor];
                    tracing::trace!(%packet_size, cursor=%self.cursor, ?buffered, "Packet size available");
                }

                if self.cursor < packet_size {
                    #[cfg(feature = "tracing")]
                    {
                        let remaining = packet_size - self.cursor;
                        tracing::trace!(%remaining, "Not enough bytes to decode the packet. Reading more bytes");
                    }

                    self.fill_buf().await?;
                }
            }

            if self.cursor < 4 {
                #[cfg(feature = "tracing")]
                {
                    tracing::trace!("Reading bytes to get packet size");
                }

                self.fill_buf().await?;
            }

            if self.cursor >= 4 {
                let packet_size =
                    u32::from_be_bytes([self.buf[0], self.buf[1], self.buf[2], self.buf[3]])
                        as usize;

                #[cfg(feature = "tracing")]
                {
                    tracing::trace!(%packet_size, "Checking if enough bytes are available");
                }

                if self.cursor < packet_size {
                    #[cfg(feature = "tracing")]
                    {
                        let remaining = packet_size - self.cursor;
                        tracing::trace!(%remaining, "Not enough bytes to decode the packet. Breaking");
                    }

                    continue;
                }

                let message_buf = &self.buf[4..packet_size];

                #[cfg(feature = "tracing")]
                {
                    let packet_buf = &self.buf[..packet_size];
                    tracing::trace!(?packet_buf, ?message_buf, "Decoding message");
                }

                // Here we have a full packet
                // Decode message starting from the 5th byte
                let message = bincode::decode_from_slice(message_buf, bincode::config::standard())
                    .map_err(DecodeError::Decode)?;

                self.cursor -= packet_size;
                self.buf.copy_within(packet_size.., 0);

                #[cfg(feature = "tracing")]
                {
                    let buffered = &self.buf[..self.cursor];
                    tracing::trace!(cursor=%self.cursor, ?buffered, "Decremented cursor");
                }

                return Ok(message.0);
            }
        }
    }

    pub fn stream(&'a mut self) -> impl Stream<Item = Result<M, DecodeError<R::Error>>> + 'a {
        futures::stream::unfold(self, |this| async {
            if this.has_errored {
                return None;
            }

            match this.read_frame().await {
                Ok(deocded) => Some((Ok(deocded), this)),
                Err(err) => {
                    this.has_errored = true;

                    Some((Err(err), this))
                }
            }
        })
    }

    pub fn into_stream<'b, 'c>(self) -> impl Stream<Item = Result<M, DecodeError<R::Error>>> + 'c
    where
        'a: 'b + 'c,
        'b: 'c,
        M: 'b,
        R: 'c,
    {
        futures::stream::unfold(self, |mut this| async {
            if this.has_errored {
                return None;
            }

            match this.read_frame().await {
                Ok(deocded) => Some((Ok(deocded), this)),
                Err(err) => {
                    this.has_errored = true;

                    Some((Err(err), this))
                }
            }
        })
    }
}
