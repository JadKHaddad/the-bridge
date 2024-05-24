extern crate std;

fn init_tracing() {
    use tracing_subscriber::fmt::format::FmtSpan;
    use tracing_subscriber::EnvFilter;

    tracing_subscriber::fmt()
        .with_env_filter(
            EnvFilter::from_default_env()
                .add_directive("trace".parse().expect("Invalid filter directive")),
        )
        .with_span_events(FmtSpan::FULL)
        .init();
}

async fn read_with_crate_stream(read: impl tokio::io::AsyncRead + Unpin) {
    use crate::{decode::framed_read::FramedRead, tokio::Compat, Message};
    use futures::StreamExt;

    let read_buf: &mut [u8] = &mut [0; 50];

    let tokio_read_compat = Compat::new(read);

    let mut reader: FramedRead<'_, _, Message> = FramedRead::new(tokio_read_compat, read_buf);
    let stream = reader.stream();

    stream
        .for_each(|r| async move { std::println!("{r:?}") })
        .await;
}

async fn read_with_crate_loop(read: impl tokio::io::AsyncRead + Unpin) {
    use crate::{decode::framed_read::FramedRead, tokio::Compat, Message};

    let read_buf: &mut [u8] = &mut [0; 50];

    let tokio_read_compat = Compat::new(read);

    let mut reader: FramedRead<'_, _, Message> = FramedRead::new(tokio_read_compat, read_buf);

    loop {
        let message = reader.read_frame().await;
        match message {
            Ok(message) => {
                std::println!("{:?}", message);
            }
            Err(error) => {
                std::println!("{:?}", error);
                break;
            }
        }
    }
}

async fn read_with_tokio_codec(read: impl tokio::io::AsyncRead + Unpin) {
    use crate::{tokio::Codec, Message};
    use futures::StreamExt;
    use tokio_util::codec::FramedRead;

    let reader = FramedRead::new(read, Codec::<Message>::default());

    reader
        .for_each(|r| async move { std::println!("{r:?}") })
        .await;
}

async fn write_slow_bytes(mut write: impl tokio::io::AsyncWrite + Unpin) {
    use tokio::io::AsyncWriteExt;

    let packet: [u8; 22] = [
        0, 0, 0, 18, 2, 200, 200, 200, 200, 200, 200, 200, 200, 200, 200, 200, 200, 200, 200, 200,
        200, 200,
    ];

    for u in packet {
        write
            .write_all(&[u])
            .await
            .expect("Failed to write to stream");

        tokio::time::sleep(tokio::time::Duration::from_millis(500)).await;
    }
}

async fn write_batch(mut write: impl tokio::io::AsyncWrite + Unpin) {
    use tokio::io::AsyncWriteExt;

    // These are 5 stacked C messages
    let packet: [u8; 110] = [
        0, 0, 0, 18, 2, 200, 200, 200, 200, 200, 200, 200, 200, 200, 200, 200, 200, 200, 200, 200,
        200, 200, 0, 0, 0, 18, 2, 200, 200, 200, 200, 200, 200, 200, 200, 200, 200, 200, 200, 200,
        200, 200, 200, 200, 0, 0, 0, 18, 2, 200, 200, 200, 200, 200, 200, 200, 200, 200, 200, 200,
        200, 200, 200, 200, 200, 200, 0, 0, 0, 18, 2, 200, 200, 200, 200, 200, 200, 200, 200, 200,
        200, 200, 200, 200, 200, 200, 200, 200, 0, 0, 0, 18, 2, 200, 200, 200, 200, 200, 200, 200,
        200, 200, 200, 200, 200, 200, 200, 200, 200, 200,
    ];

    write
        .write_all(&packet)
        .await
        .expect("Failed to write to stream");

    tokio::time::sleep(tokio::time::Duration::from_secs(3)).await;
}

async fn write_with_crate_loop(write: impl tokio::io::AsyncWrite + Unpin) {
    use crate::{encode::framed_write::FramedWrite, tokio::Compat, Message};

    let mut buf = [0; 100];

    let tokio_write_compat = Compat::new(write);
    let mut writer = FramedWrite::new(tokio_write_compat, &mut buf);

    for _ in 0..10 {
        let message = Message::C(
            100, 100, 100, 100, 100, 100, 100, 100, 100, 100, 100, 100, 100, 100, 100, 100, 100,
        );

        writer
            .write_frame(&message)
            .await
            .expect("Failed to write packet");

        tokio::time::sleep(tokio::time::Duration::from_secs(1)).await;
    }
}

#[tokio::test]
#[ignore]
// cargo test --package bincode-bridge --lib -- tests::send_slow_bytes_to_crates --exact --show-output --ignored --nocapture
async fn send_slow_bytes_to_crate_stream() {
    init_tracing();

    let (read, write) = tokio::io::duplex(1024);

    let read_task = tokio::spawn(read_with_crate_stream(read));
    let write_task = tokio::spawn(write_slow_bytes(write));

    tokio::try_join!(read_task, write_task).expect("Failed to join tasks");
}

#[tokio::test]
#[ignore]
// cargo test --package bincode-bridge --lib -- tests::send_slow_bytes_to_crate_loop --exact --show-output --ignored --nocapture
async fn send_slow_bytes_to_crate_loop() {
    init_tracing();

    let (read, write) = tokio::io::duplex(1024);

    let read_task = tokio::spawn(read_with_crate_loop(read));
    let write_task = tokio::spawn(write_slow_bytes(write));

    tokio::try_join!(read_task, write_task).expect("Failed to join tasks");
}

#[tokio::test]
#[ignore]
// cargo test --package bincode-bridge --lib -- tests::send_slow_bytes_to_tokio_codec --exact --show-output --ignored --nocapture
async fn send_slow_bytes_to_tokio_codec() {
    init_tracing();

    let (read, write) = tokio::io::duplex(1024);

    let read_task = tokio::spawn(read_with_tokio_codec(read));
    let write_task = tokio::spawn(write_slow_bytes(write));

    tokio::try_join!(read_task, write_task).expect("Failed to join tasks");
}

#[tokio::test]
#[ignore]
async fn do_stuff() {
    use crate::{decode::framed_read::FramedRead, tokio::Compat, Message};
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

    let mut reader: FramedRead<'_, _, Message> = FramedRead::new(tokio_read_compat, read_buf);
    let stream = reader.stream();

    stream
        .for_each(|r| async move { std::println!("{r:?}") })
        .await;
}
