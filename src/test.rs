extern crate std;
use std::{boxed::Box, string::String, vec::Vec};

#[derive(Debug, Clone, bincode::Encode, bincode::Decode, PartialEq)]
pub enum TestMessage {
    A(u8),
    B(i32),
    C(i64, i64),
    D(u128, u128, u128),
    E(String),
    F(Vec<TestMessage>),
    G(Box<TestMessage>),
    H,
    I(
        i32,
        i32,
        i32,
        i32,
        i32,
        i32,
        i32,
        i32,
        i32,
        i32,
        i32,
        i32,
        i32,
        i32,
        i32,
        i32,
        i32,
    ),
    Z(
        u8,
        i32,
        i64,
        u128,
        String,
        Vec<TestMessage>,
        Box<TestMessage>,
    ),
}

pub fn z_test_message() -> TestMessage {
    TestMessage::Z(
        1,
        2,
        3,
        4,
        String::from("Hello"),
        std::vec![
            TestMessage::A(100),
            TestMessage::B(100),
            TestMessage::C(100, 100),
            TestMessage::D(100, 100, 100),
        ],
        Box::new(TestMessage::A(100)),
    )
}

pub fn test_messages() -> Vec<TestMessage> {
    std::vec![
        TestMessage::A(100),
        TestMessage::B(100),
        TestMessage::C(100, 100),
        TestMessage::D(100, 100, 100),
        TestMessage::E(String::from("Hello")),
        TestMessage::F(std::vec![
            TestMessage::A(100),
            TestMessage::B(100),
            TestMessage::C(100, 100),
            TestMessage::D(100, 100, 100),
        ]),
        TestMessage::G(Box::new(TestMessage::A(100))),
        TestMessage::H,
        TestMessage::I(1, 2, 3, 4, 5, 6, 7, 8, 9, 10, 11, 12, 13, 14, 15, 16, 17),
        z_test_message(),
    ]
}

#[cfg(all(feature = "tokio", feature = "cody-c"))]
mod comp {
    use cody_c::{tokio::Compat, FramedRead as CodyFramedRead, FramedWrite as CodyFramedWrite};
    use futures::{pin_mut, stream, SinkExt, StreamExt};
    use tokio_util::codec::{FramedRead as TokioFramedRead, FramedWrite as TokioFramedWrite};

    use super::*;
    use crate::codec::Codec;

    #[tokio::test]
    async fn cody_sink_tokio_stream() {
        let items = test_messages();

        let (read, write) = tokio::io::duplex(16);

        let handle = tokio::spawn(async move {
            let codec = Codec::<TestMessage>::new();
            let mut framed_write =
                CodyFramedWrite::new_with_buffer(codec, Compat::new(write), [0_u8; 128]);
            let framed_write = framed_write.sink();

            pin_mut!(framed_write);

            for item in items {
                framed_write.send(item).await.unwrap();
            }

            framed_write.close().await.unwrap();
        });

        let codec = Codec::<TestMessage>::new();
        let framed_read = TokioFramedRead::new(read, codec);

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

    #[tokio::test]
    async fn tokio_sink_cody_stream() {
        let items = test_messages();

        let (read, write) = tokio::io::duplex(16);

        let handle = tokio::spawn(async move {
            let codec = Codec::<TestMessage>::new();
            let mut framed_write = TokioFramedWrite::new(write, codec);

            framed_write
                .send_all(&mut stream::iter(items.into_iter().map(Ok)))
                .await
                .unwrap();

            framed_write.close().await.unwrap();
        });

        let codec = Codec::<TestMessage>::new();
        let mut framed_read =
            CodyFramedRead::new_with_buffer(codec, Compat::new(read), [0_u8; 128]);
        let framed_read = framed_read.stream();

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
