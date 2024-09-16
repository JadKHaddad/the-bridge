use cody_c::{Decoder, Encoder, Frame, FrameSize, MaybeDecoded};

use crate::codec::Codec;

#[derive(Debug)]
#[non_exhaustive]
pub enum EncodeError {
    InputBufferTooSmall,
    Encode(bincode::error::EncodeError),
    MessageTooBig,
}

impl<M> Encoder<M> for Codec<M>
where
    M: bincode::Encode,
{
    type Error = EncodeError;

    fn encode(&mut self, item: M, dst: &mut [u8]) -> Result<usize, Self::Error> {
        if dst.len() < 4 {
            return Err(EncodeError::InputBufferTooSmall);
        }

        let message_size =
            bincode::encode_into_slice(item, &mut dst[4..], bincode::config::standard())
                .map_err(EncodeError::Encode)?;

        // TODO: make this a feature or a configuration option
        if message_size > u32::MAX as usize {
            return Err(EncodeError::MessageTooBig);
        }

        let packet_size = message_size as u32 + 4;
        let packet_size_bytes = packet_size.to_be_bytes();
        dst[0..4].copy_from_slice(&packet_size_bytes);

        Ok(packet_size as usize)
    }
}

#[derive(Debug)]
pub enum DecodeError {
    InvalidFrameSize,
    Decode(bincode::error::DecodeError),
}

impl<M> Decoder for Codec<M>
where
    M: bincode::Decode,
{
    type Item = M;

    type Error = DecodeError;

    fn decode(&mut self, src: &mut [u8]) -> Result<MaybeDecoded<Self::Item>, Self::Error> {
        if src.len() < 4 {
            return Ok(MaybeDecoded::None(FrameSize::Unknown));
        }

        let frame_size = u32::from_be_bytes([src[0], src[1], src[2], src[3]]) as usize;

        if src.len() < frame_size {
            return Ok(MaybeDecoded::None(FrameSize::Known(frame_size)));
        }

        if frame_size < 4 {
            return Err(DecodeError::InvalidFrameSize);
        }

        let (item, _) =
            bincode::decode_from_slice(&src[4..frame_size], bincode::config::standard())
                .map_err(DecodeError::Decode)?;

        Ok(MaybeDecoded::Frame(Frame::new(frame_size, item)))
    }
}

#[cfg(test)]
mod test {
    extern crate std;
    use std::vec::Vec;

    use cody_c::{tokio::Compat, FramedRead, FramedWrite};
    use futures::{SinkExt, StreamExt};

    use crate::{
        codec::Codec,
        test::{test_messages, TestMessage},
    };

    #[tokio::test]
    async fn sink_stream() {
        let items = test_messages();

        let (read, write) = tokio::io::duplex(16);

        let handle = tokio::spawn(async move {
            let write_buf = &mut [0_u8; 128];
            let codec = Codec::<TestMessage>::new();
            let mut framed_write = FramedWrite::new(Compat::new(write), codec, write_buf);

            for item in items {
                framed_write.send(item).await.unwrap();
            }

            framed_write.close().await.unwrap();
        });

        let read_buf = &mut [0_u8; 128];
        let codec = Codec::<TestMessage>::new();
        let framed_read = FramedRead::new(Compat::new(read), codec, read_buf);

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
