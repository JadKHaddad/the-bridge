use core::hint::black_box;
use criterion::{criterion_group, criterion_main, Criterion};

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

mod cody_c {
    use cody_c::{decode::framed_read::FramedRead, tokio::Compat, FramedWrite};
    use futures::{SinkExt, StreamExt};
    use the_bridge::Codec;

    use crate::TestMessage;

    pub fn bench<const BUF_SIZE: usize>(items: Vec<TestMessage>, duplex_size: usize) {
        tokio::runtime::Builder::new_multi_thread()
            .enable_all()
            .build()
            .unwrap()
            .block_on(async {
                let items_clone = items.clone();

                let (read, write) = tokio::io::duplex(duplex_size);

                let handle = tokio::spawn(async move {
                    let write_buf = &mut [0_u8; BUF_SIZE];
                    let codec = the_bridge::codec::Codec::<TestMessage>::new();
                    let mut framed_write = FramedWrite::new(Compat::new(write), codec, write_buf);

                    for item in items_clone {
                        framed_write.send(item).await.unwrap();
                    }

                    framed_write.close().await.unwrap();
                });

                let read_buf = &mut [0u8; BUF_SIZE];
                let codec = Codec::<TestMessage>::new();
                let framed_read = FramedRead::new(Compat::new(read), codec, read_buf);

                let collected_items: Vec<_> = framed_read
                    .collect::<Vec<_>>()
                    .await
                    .into_iter()
                    .flatten()
                    .collect::<Vec<_>>();

                handle.await.unwrap();

                assert_eq!(collected_items, items);
            })
    }
}

mod tokio_codec {
    use futures::{SinkExt, StreamExt};
    use the_bridge::Codec;
    use tokio_util::codec::{FramedRead, FramedWrite};

    use crate::TestMessage;

    pub fn bench(items: Vec<TestMessage>, duplex_size: usize) {
        tokio::runtime::Builder::new_multi_thread()
            .enable_all()
            .build()
            .unwrap()
            .block_on(async {
                let items_clone = items.clone();

                let (read, write) = tokio::io::duplex(duplex_size);

                let handle = tokio::spawn(async move {
                    let codec = Codec::<TestMessage>::new();
                    let mut framed_write = FramedWrite::new(write, codec);

                    for item in items_clone {
                        framed_write.send(item).await.unwrap();
                    }

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

                assert_eq!(collected_items, items);
            })
    }
}

fn criterion_benchmark(c: &mut Criterion) {
    let duplex_size: usize = 1024;

    let test_messages = (0..10).fold(Vec::new(), |mut acc, _| {
        acc.extend(test_messages());

        acc
    });

    c.bench_function("cody_c_buf_32", |b| {
        b.iter(|| cody_c::bench::<32>(black_box(test_messages.clone()), black_box(duplex_size)))
    });
    c.bench_function("cody_c_buf_1024", |b| {
        b.iter(|| cody_c::bench::<1024>(black_box(test_messages.clone()), black_box(duplex_size)))
    });
    c.bench_function("tokio_codec", |b| {
        b.iter(|| tokio_codec::bench(black_box(test_messages.clone()), black_box(duplex_size)))
    });
}

criterion_group!(benches, criterion_benchmark);
criterion_main!(benches);
