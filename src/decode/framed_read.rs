use super::{async_read::AsyncRead, error::DecodeError};
use core::marker::PhantomData;

pub struct FramedRead<'a, R, M> {
    reader: R,
    buf: &'a mut [u8],
    cursor: usize,
    #[cfg(feature = "futures")]
    has_errored: bool,
    _phantom: PhantomData<M>,
}

impl<'a, R, M> AsRef<R> for FramedRead<'a, R, M> {
    fn as_ref(&self) -> &R {
        &self.reader
    }
}

impl<'a, R, M> AsMut<R> for FramedRead<'a, R, M> {
    fn as_mut(&mut self) -> &mut R {
        &mut self.reader
    }
}

impl<'a, R: AsyncRead, M: bincode::Decode> FramedRead<'a, R, M> {
    pub fn new(reader: R, buf: &'a mut [u8]) -> Self {
        Self {
            reader,
            buf,
            cursor: 0,
            #[cfg(feature = "futures")]
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
        tracing::trace!(%read);

        #[cfg(feature = "log")]
        log::info!("Read {} bytes", read);

        #[cfg(feature = "defmt")]
        defmt::info!("Read {} bytes", read);

        if read == 0 {
            if self.cursor == buf.len() {
                // The packet is too big
                return Err(DecodeError::BufferIsFull);
            }

            // Got EOF
            return Err(DecodeError::ReadZero);
        }

        self.cursor += read;

        #[cfg(any(feature = "tracing", feature = "log", feature = "defmt"))]
        let buffered = &self.buf[..self.cursor];

        #[cfg(feature = "tracing")]
        tracing::trace!(cursor=%self.cursor, ?buffered, "Incremented cursor");

        #[cfg(feature = "log")]
        log::info!("Incremented cursor. buffered: {:?}", buffered);

        #[cfg(feature = "defmt")]
        defmt::info!("Incremented cursor. buffered: {:?}", buffered);

        Ok(())
    }

    // FIXME: find a better way to shift the buffer thatn copy_within
    pub async fn read_frame(&mut self) -> Result<M, DecodeError<R::Error>> {
        loop {
            if self.cursor >= 4 {
                let packet_size =
                    u32::from_be_bytes([self.buf[0], self.buf[1], self.buf[2], self.buf[3]])
                        as usize;

                #[cfg(any(feature = "tracing", feature = "log", feature = "defmt"))]
                let buffered = &self.buf[..self.cursor];

                #[cfg(feature = "tracing")]
                tracing::trace!(%packet_size, cursor=%self.cursor, ?buffered, "Packet size available");

                #[cfg(feature = "log")]
                log::info!(
                    "Packet size available. packet_size: {}, cursor: {}, buffered: {:?}",
                    packet_size,
                    self.cursor,
                    buffered
                );

                #[cfg(feature = "defmt")]
                defmt::info!(
                    "Packet size available. packet_size: {}, cursor: {}, buffered: {:?}",
                    packet_size,
                    self.cursor,
                    buffered
                );

                if self.cursor < packet_size {
                    #[cfg(any(feature = "tracing", feature = "log", feature = "defmt"))]
                    let remaining = packet_size - self.cursor;

                    #[cfg(feature = "tracing")]
                    tracing::trace!(%remaining, "Not enough bytes to decode the packet. Reading more bytes");

                    #[cfg(feature = "log")]
                    log::info!(
                        "Not enough bytes to decode the packet. Reading more bytes. remaining: {}",
                        remaining
                    );

                    #[cfg(feature = "defmt")]
                    defmt::info!(
                        "Not enough bytes to decode the packet. Reading more bytes. remaining: {}",
                        remaining
                    );

                    self.fill_buf().await?;
                }
            }

            if self.cursor < 4 {
                #[cfg(feature = "tracing")]
                tracing::trace!("Reading bytes to get packet size");

                #[cfg(feature = "log")]
                log::info!("Reading bytes to get packet size");

                #[cfg(feature = "defmt")]
                defmt::info!("Reading bytes to get packet size");

                self.fill_buf().await?;
            }

            if self.cursor >= 4 {
                let packet_size =
                    u32::from_be_bytes([self.buf[0], self.buf[1], self.buf[2], self.buf[3]])
                        as usize;

                #[cfg(feature = "tracing")]
                tracing::trace!(%packet_size, "Checking if enough bytes are available");

                #[cfg(feature = "log")]
                log::info!(
                    "Checking if enough bytes are available. packet_size: {}",
                    packet_size
                );

                #[cfg(feature = "defmt")]
                defmt::info!(
                    "Checking if enough bytes are available. packet_size: {}",
                    packet_size
                );

                if self.cursor < packet_size {
                    #[cfg(any(feature = "tracing", feature = "log", feature = "defmt"))]
                    let remaining = packet_size - self.cursor;

                    #[cfg(feature = "tracing")]
                    tracing::trace!(%remaining, "Not enough bytes to decode the packet. Breaking");

                    #[cfg(feature = "log")]
                    log::info!(
                        "Not enough bytes to decode the packet. Breaking. remaining: {}",
                        remaining
                    );

                    #[cfg(feature = "defmt")]
                    defmt::info!(
                        "Not enough bytes to decode the packet. Breaking. remaining: {}",
                        remaining
                    );

                    continue;
                }

                let message_buf = &self.buf[4..packet_size];

                #[cfg(any(feature = "tracing", feature = "log", feature = "defmt"))]
                let packet_buf = &self.buf[..packet_size];

                #[cfg(feature = "tracing")]
                tracing::trace!(?packet_buf, ?message_buf, "Decoding message");

                #[cfg(feature = "log")]
                log::info!(
                    "Decoding message. packet_buf: {:?}, message_buf: {:?}",
                    packet_buf,
                    message_buf
                );

                #[cfg(feature = "defmt")]
                defmt::info!(
                    "Decoding message. packet_buf: {:?}, message_buf: {:?}",
                    packet_buf,
                    message_buf
                );

                // Here we have a full packet
                // Decode message starting from the 5th byte
                let message = bincode::decode_from_slice(message_buf, bincode::config::standard())
                    .map_err(DecodeError::Decode)?;

                self.cursor -= packet_size;
                self.buf.copy_within(packet_size.., 0);

                #[cfg(any(feature = "tracing", feature = "log", feature = "defmt"))]
                let buffered = &self.buf[..self.cursor];

                #[cfg(feature = "tracing")]
                tracing::trace!(cursor=%self.cursor, ?buffered, "Decremented cursor");

                #[cfg(feature = "log")]
                log::info!(
                    "Decremented cursor. cursor: {}, buffered: {:?}",
                    self.cursor,
                    buffered
                );

                #[cfg(feature = "defmt")]
                defmt::info!(
                    "Decremented cursor. cursor: {}, buffered: {:?}",
                    self.cursor,
                    buffered
                );

                return Ok(message.0);
            }
        }
    }
}

#[cfg(feature = "futures")]
const _: () = {
    use crate::captures::Captures;
    use futures::Stream;

    impl<'a, R: AsyncRead, M: bincode::Decode> FramedRead<'a, R, M> {
        pub fn stream(
            &'a mut self,
        ) -> impl Stream<Item = Result<M, DecodeError<R::Error>>> + Captures<&'a Self> {
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

        pub fn into_stream(
            self,
        ) -> impl Stream<Item = Result<M, DecodeError<R::Error>>> + Captures<&'a Self> {
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
};
