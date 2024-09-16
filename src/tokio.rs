use crate::codec::Codec;
use tokio_util::{
    bytes::{Buf, BufMut, BytesMut},
    codec::{Decoder, Encoder},
};

#[derive(Debug)]
#[non_exhaustive]
pub enum EncodeError {
    IO(std::io::Error),
    Encode(bincode::error::EncodeError),
    MessageTooBig,
}

impl From<std::io::Error> for EncodeError {
    fn from(err: std::io::Error) -> Self {
        EncodeError::IO(err)
    }
}

impl<M> Encoder<M> for Codec<M>
where
    M: bincode::Encode,
{
    type Error = EncodeError;

    fn encode(&mut self, item: M, dst: &mut BytesMut) -> Result<(), Self::Error> {
        let start_len = dst.len();

        dst.put_u32(0);

        let message_size =
            bincode::encode_into_std_write(item, &mut dst.writer(), bincode::config::standard())
                .map_err(EncodeError::Encode)?;

        // TODO: make this a feature or a configuration option
        if message_size > u32::MAX as usize {
            return Err(EncodeError::MessageTooBig);
        }

        let packet_size = (dst.len() - start_len) as u32;
        let packet_size_bytes = packet_size.to_be_bytes();

        dst[start_len..start_len + 4].copy_from_slice(&packet_size_bytes);

        Ok(())
    }
}

#[derive(Debug)]
pub enum DecodeError {
    IO(std::io::Error),
    InvalidFrameSize,
    Decode(bincode::error::DecodeError),
}

impl From<std::io::Error> for DecodeError {
    fn from(err: std::io::Error) -> Self {
        DecodeError::IO(err)
    }
}

impl<M> Decoder for Codec<M>
where
    M: bincode::Decode,
{
    type Item = M;
    type Error = DecodeError;

    fn decode(&mut self, src: &mut BytesMut) -> Result<Option<Self::Item>, Self::Error> {
        if src.len() < 4 {
            return Ok(None);
        }

        let packet_size = u32::from_be_bytes([src[0], src[1], src[2], src[3]]) as usize;

        if src.len() < packet_size {
            src.reserve(packet_size - src.len());

            return Ok(None);
        }

        if packet_size < 4 {
            return Err(DecodeError::InvalidFrameSize);
        }

        let message_buf = &src[4..packet_size];
        let message = bincode::decode_from_slice(message_buf, bincode::config::standard())
            .map_err(DecodeError::Decode)?;

        src.advance(packet_size);

        Ok(Some(message.0))
    }
}

#[cfg(test)]
mod test {
    use futures::{stream, SinkExt, StreamExt};
    use tokio_util::codec::{FramedRead, FramedWrite};

    use crate::{
        codec::Codec,
        test::{test_messages, TestMessage},
    };

    #[tokio::test]
    async fn sink_stream() {
        let items = test_messages();

        let (read, write) = tokio::io::duplex(16);

        let handle = tokio::spawn(async move {
            let codec = Codec::<TestMessage>::new();
            let mut framed_write = FramedWrite::new(write, codec);

            framed_write
                .send_all(&mut stream::iter(items.into_iter().map(Ok)))
                .await
                .unwrap();

            framed_write.close().await.unwrap();
        });

        let codec = Codec::<TestMessage>::new();
        let framed_read = FramedRead::new(read, codec);

        let collected_items: Vec<_> = framed_read
            .collect::<Vec<_>>()
            .await
            .into_iter()
            .flatten()
            .collect::<Vec<_>>();

        handle.await.unwrap();

        let items = test_messages();

        assert_eq!(collected_items, items);
    }
}
