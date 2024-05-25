extern crate std;

use std::{boxed::Box, string::String, vec::Vec};

#[derive(Debug, Clone, bincode::Encode, bincode::Decode, PartialEq)]
enum TestMessage {
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

fn z_test_message() -> TestMessage {
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

fn test_messages() -> Vec<TestMessage> {
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

async fn encoded_packet_of_z_test_message() -> Vec<u8> {
    use crate::encode::framed_write::FramedWrite;
    use crate::tokio::Compat;

    let mut packet: Vec<u8> = Vec::new();
    let message = z_test_message();

    let mut buf: &mut [u8] = &mut [0; 1024];
    let mut writer = FramedWrite::new(Compat::new(&mut packet), &mut buf);

    writer
        .write_frame(&message)
        .await
        .expect("Failed to write packet");

    packet
}

async fn encoded_packets_of_test_messages() -> Vec<u8> {
    use crate::encode::framed_write::FramedWrite;
    use crate::tokio::Compat;

    let mut packets: Vec<u8> = Vec::new();
    let messages = test_messages();

    let mut buf: &mut [u8] = &mut [0; 1024];
    let mut writer = FramedWrite::new(Compat::new(&mut packets), &mut buf);

    for message in messages {
        writer
            .write_frame(&message)
            .await
            .expect("Failed to write packet");
    }

    packets
}

#[tokio::test]
#[ignore]
async fn print_encoded_packet_of_z_test_message() {
    let packet = encoded_packet_of_z_test_message().await;

    std::println!("{:?}", packet);
}

#[tokio::test]
#[ignore]
async fn print_encoded_packets_of_test_messages() {
    let packets = encoded_packets_of_test_messages().await;

    std::println!("{:?}", packets);
}

fn init_tracing() {
    use tracing_subscriber::{fmt::format::FmtSpan, EnvFilter};

    let _ = tracing_subscriber::fmt()
        .with_env_filter(
            EnvFilter::from_default_env()
                .add_directive("trace".parse().expect("Invalid filter directive")),
        )
        .with_span_events(FmtSpan::FULL)
        .try_init();
}

async fn read_with_crate_stream(read: impl tokio::io::AsyncRead + Unpin) -> Vec<TestMessage> {
    use crate::{decode::framed_read::FramedRead, tokio::Compat};
    use futures::StreamExt;

    let buf: &mut [u8] = &mut [0; 50];

    let mut reader: FramedRead<'_, _, TestMessage> = FramedRead::new(Compat::new(read), buf);
    let stream = reader.stream();

    stream
        .collect::<Vec<_>>()
        .await
        .into_iter()
        .flatten()
        .collect::<Vec<_>>()
}

async fn read_with_crate_loop(read: impl tokio::io::AsyncRead + Unpin) -> Vec<TestMessage> {
    use crate::{decode::framed_read::FramedRead, tokio::Compat};

    let buf: &mut [u8] = &mut [0; 50];

    let mut reader: FramedRead<'_, _, TestMessage> = FramedRead::new(Compat::new(read), buf);

    let mut messages = Vec::new();

    loop {
        let message = reader.read_frame().await;
        match message {
            Ok(message) => {
                messages.push(message);
            }
            Err(error) => {
                std::println!("{:?}", error);
                break;
            }
        }
    }

    messages
}

async fn read_with_tokio_codec(read: impl tokio::io::AsyncRead + Unpin) -> Vec<TestMessage> {
    use crate::tokio::Codec;
    use futures::StreamExt;
    use tokio_util::codec::FramedRead;

    let reader = FramedRead::new(read, Codec::<TestMessage>::default());

    reader
        .collect::<Vec<_>>()
        .await
        .into_iter()
        .flatten()
        .collect::<Vec<_>>()
}

// -----------------------------------------------------------------------------

async fn raw_write_slow_bytes_of_z_message(mut write: impl tokio::io::AsyncWrite + Unpin) {
    use tokio::io::AsyncWriteExt;

    let packet = encoded_packet_of_z_test_message().await;

    for u in packet {
        write
            .write_all(&[u])
            .await
            .expect("Failed to write to stream");

        tokio::time::sleep(tokio::time::Duration::from_millis(500)).await;
    }
}

async fn raw_write_test_messages_batch(mut write: impl tokio::io::AsyncWrite + Unpin) {
    use tokio::io::AsyncWriteExt;

    let packets = encoded_packets_of_test_messages().await;

    write
        .write_all(&packets)
        .await
        .expect("Failed to write to stream");

    tokio::time::sleep(tokio::time::Duration::from_secs(1)).await;
}

// -----------------------------------------------------------------------------

async fn write_with_crate_loop(
    write: impl tokio::io::AsyncWrite + Unpin,
    messages: Vec<TestMessage>,
) {
    use crate::{encode::framed_write::FramedWrite, tokio::Compat};

    let mut buf = [0; 100];
    let mut writer = FramedWrite::new(Compat::new(write), &mut buf);

    for message in messages {
        writer
            .write_frame(&message)
            .await
            .expect("Failed to write packet");
    }
}

async fn write_with_crate_sink(
    write: impl tokio::io::AsyncWrite + Unpin,
    messages: Vec<TestMessage>,
) {
    use crate::{encode::framed_write::FramedWrite, tokio::Compat};
    use futures::SinkExt;

    let buf = &mut [0; 100];
    let mut writer: FramedWrite<'_, _, TestMessage> = FramedWrite::new(Compat::new(write), buf);

    let sink = writer.sink();
    futures::pin_mut!(sink);

    for message in messages.clone() {
        sink.send(message).await.expect("Failed to send message");
    }
}

async fn write_with_tokio_codec(
    write: impl tokio::io::AsyncWrite + Unpin,
    messages: Vec<TestMessage>,
) {
    use crate::tokio::Codec;
    use futures::SinkExt;
    use tokio_util::codec::FramedWrite;

    let mut sink = FramedWrite::new(write, Codec::<TestMessage>::default());

    for message in messages.clone() {
        sink.send(message).await.expect("Failed to send message");
    }

    sink.close().await.expect("Failed to close sink");

    tracing::info!("Sink closed");
}

// -----------------------------------------------------------------------------

#[tokio::test]
async fn send_slow_bytes_to_crate_stream() {
    init_tracing();

    let (read, write) = tokio::io::duplex(1024);

    let read_task = tokio::spawn(read_with_crate_stream(read));
    let write_task = tokio::spawn(raw_write_slow_bytes_of_z_message(write));

    let (messages_read, _) = tokio::try_join!(read_task, write_task).expect("Failed to join tasks");

    assert_eq!(messages_read[0], z_test_message());
}

#[tokio::test]
async fn send_batch_to_crate_stream() {
    init_tracing();

    let (read, write) = tokio::io::duplex(1024);

    let read_task = tokio::spawn(read_with_crate_stream(read));
    let write_task = tokio::spawn(raw_write_test_messages_batch(write));

    let (messages_read, _) = tokio::try_join!(read_task, write_task).expect("Failed to join tasks");

    assert_eq!(messages_read, test_messages());
}

#[tokio::test]
async fn send_slow_bytes_to_crate_loop() {
    init_tracing();

    let (read, write) = tokio::io::duplex(1024);

    let read_task = tokio::spawn(read_with_crate_loop(read));
    let write_task = tokio::spawn(raw_write_slow_bytes_of_z_message(write));

    let (messages_read, _) = tokio::try_join!(read_task, write_task).expect("Failed to join tasks");

    assert_eq!(messages_read[0], z_test_message());
}

#[tokio::test]
async fn send_batch_to_create_loop() {
    init_tracing();

    let (read, write) = tokio::io::duplex(1024);

    let read_task = tokio::spawn(read_with_crate_loop(read));
    let write_task = tokio::spawn(raw_write_test_messages_batch(write));

    let (messages_read, _) = tokio::try_join!(read_task, write_task).expect("Failed to join tasks");

    assert_eq!(messages_read, test_messages());
}

#[tokio::test]
async fn send_slow_bytes_to_tokio_codec() {
    init_tracing();

    let (read, write) = tokio::io::duplex(1024);

    let read_task = tokio::spawn(read_with_tokio_codec(read));
    let write_task = tokio::spawn(raw_write_slow_bytes_of_z_message(write));

    let (messages_read, _) = tokio::try_join!(read_task, write_task).expect("Failed to join tasks");

    assert_eq!(messages_read[0], z_test_message());
}

#[tokio::test]
async fn send_batch_to_tokio_codec() {
    init_tracing();

    let (read, write) = tokio::io::duplex(1024);

    let read_task = tokio::spawn(read_with_tokio_codec(read));
    let write_task = tokio::spawn(raw_write_test_messages_batch(write));

    let (messages_read, _) = tokio::try_join!(read_task, write_task).expect("Failed to join tasks");

    assert_eq!(messages_read, test_messages());
}

// -----------------------------------------------------------------------------

#[tokio::test]
async fn crate_loop_crate_stream() {
    init_tracing();

    let messages = test_messages();
    let (read, write) = tokio::io::duplex(1024);

    let write_task = write_with_crate_loop(write, messages.clone());
    let read_task = read_with_crate_stream(read);

    let (_, messages_read) = tokio::join!(write_task, read_task);

    assert_eq!(messages, messages_read);
}

#[tokio::test]
async fn crate_loop_crate_loop() {
    init_tracing();

    let messages = test_messages();
    let (read, write) = tokio::io::duplex(1024);

    let write_task = write_with_crate_loop(write, messages.clone());
    let read_task = read_with_crate_loop(read);

    let (_, messages_read) = tokio::join!(write_task, read_task);

    assert_eq!(messages, messages_read);
}

#[tokio::test]
async fn crate_loop_tokio_stream() {
    init_tracing();

    let messages = test_messages();
    let (read, write) = tokio::io::duplex(1024);

    let write_task = write_with_crate_loop(write, messages.clone());
    let read_task = read_with_tokio_codec(read);

    let (_, messages_read) = tokio::join!(write_task, read_task);

    assert_eq!(messages, messages_read);
}

#[tokio::test]
async fn crate_sink_crate_stream() {
    init_tracing();

    let messages = test_messages();
    let (read, write) = tokio::io::duplex(1024);

    let write_task = write_with_crate_sink(write, messages.clone());
    let read_task = read_with_crate_stream(read);

    let (_, messages_read) = tokio::join!(write_task, read_task);

    assert_eq!(messages, messages_read);
}

#[tokio::test]
async fn crate_sink_crate_loop() {
    init_tracing();

    let messages = test_messages();
    let (read, write) = tokio::io::duplex(1024);

    let write_task = write_with_crate_sink(write, messages.clone());
    let read_task = read_with_crate_loop(read);

    let (_, messages_read) = tokio::join!(write_task, read_task);

    assert_eq!(messages, messages_read);
}

#[tokio::test]
async fn crate_sink_tokio_stream() {
    init_tracing();

    let messages = test_messages();
    let (read, write) = tokio::io::duplex(1024);

    let write_task = write_with_crate_sink(write, messages.clone());
    let read_task = read_with_tokio_codec(read);

    let (_, messages_read) = tokio::join!(write_task, read_task);

    assert_eq!(messages, messages_read);
}

#[tokio::test]
async fn tokio_sink_crate_stream() {
    init_tracing();

    let messages = test_messages();
    let (read, write) = tokio::io::duplex(1024);

    let write_task = write_with_tokio_codec(write, messages.clone());
    let read_task = read_with_crate_stream(read);

    let (_, messages_read) = tokio::join!(write_task, read_task);

    assert_eq!(messages, messages_read);
}

#[tokio::test]
async fn tokio_sink_crate_loop() {
    init_tracing();

    let messages = test_messages();
    let (read, write) = tokio::io::duplex(1024);

    let write_task = write_with_tokio_codec(write, messages.clone());
    let read_task = read_with_crate_loop(read);

    let (_, messages_read) = tokio::join!(write_task, read_task);

    assert_eq!(messages, messages_read);
}

#[tokio::test]
async fn tokio_sink_tokio_stream() {
    init_tracing();

    let messages = test_messages();

    let (read, write) = tokio::io::duplex(1024);

    let write_task = write_with_tokio_codec(write, messages.clone());
    let read_task = read_with_tokio_codec(read);

    let (_, messages_read) = tokio::join!(write_task, read_task);

    assert_eq!(messages, messages_read);
}

// -----------------------------------------------------------------------------

#[tokio::test]
#[ignore]
async fn do_stuff() {
    use crate::{decode::framed_read::FramedRead, tokio::Compat};
    use core::time::Duration;
    use futures::StreamExt;
    use std::io::{self};

    let mock_stream = tokio_test::io::Builder::new()
        .read(&[
            0, 0, 0, 18, 2, 200, 200, 200, 200, 200, 200, 200, 200, 200, 200, 200, 200, 200, 200,
            200, 200, 200,
        ])
        .wait(Duration::from_secs(1))
        .read(&[
            0, 0, 0, 18, 2, 200, 200, 200, 200, 200, 200, 200, 200, 200, 200, 200, 200, 200, 200,
            200, 200, 200,
        ])
        .wait(Duration::from_secs(1))
        .read(&[
            0, 0, 0, 18, 2, 200, 200, 200, 200, 200, 200, 200, 200, 200, 200, 200, 200, 200, 200,
            200, 200, 200,
        ])
        .wait(Duration::from_secs(1))
        .read(&[
            0, 0, 0, 18, 2, 200, 200, 200, 200, 200, 200, 200, 200, 200, 200, 200, 200, 200, 200,
            200, 200, 200,
        ])
        .wait(Duration::from_secs(1))
        .read(&[
            0, 0, 0, 18, 2, 200, 12, 12, 200, 12, 200, 200, 200, 200, 200, 200, 200, 200, 200, 200,
            200, 200,
        ])
        .wait(Duration::from_secs(1))
        .read_error(io::Error::from(io::ErrorKind::Other))
        .build();

    let read_buf: &mut [u8] = &mut [0; 23];

    let tokio_read_compat = Compat::new(mock_stream);

    let mut reader: FramedRead<'_, _, TestMessage> = FramedRead::new(tokio_read_compat, read_buf);
    let stream = reader.stream();

    stream
        .for_each(|r| async move { std::println!("{r:?}") })
        .await;
}

// TODO: add tests to buffer size errors
