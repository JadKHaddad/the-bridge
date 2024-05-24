use bincode_bridge::{
    decode::framed_read::FramedRead, demo::DemoMessage, encode::framed_write::FramedWrite,
    tokio::Compat,
};
use futures::SinkExt;
use futures::StreamExt;

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    if std::env::var_os("RUST_LOG").is_none() {
        std::env::set_var("RUST_LOG", "trace");
    }

    tracing_subscriber::fmt()
        .with_env_filter(tracing_subscriber::EnvFilter::from_default_env())
        .init();

    let addr = "0.0.0.0:5000";

    tracing::info!(%addr, "Starting server");

    let listener = tokio::net::TcpListener::bind(addr).await?;

    loop {
        let (socket, _) = listener.accept().await?;

        tracing::debug!("Connected");

        tokio::spawn(async move {
            let (reader, writer) = socket.into_split();

            let read_buf: &mut [u8] = &mut [0; 100];
            let write_buf: &mut [u8] = &mut [0; 100];

            let mut reader: FramedRead<'_, _, DemoMessage> =
                FramedRead::new(Compat::new(reader), read_buf);
            let stream = reader.stream();
            futures::pin_mut!(stream);

            let mut writer: FramedWrite<'_, _, DemoMessage> =
                FramedWrite::new(Compat::new(writer), write_buf);

            let sink = writer.sink();
            futures::pin_mut!(sink);

            let mut ping_count = 0;
            loop {
                tokio::select! {
                    _ = tokio::time::sleep(std::time::Duration::from_secs(5)) => {
                        let ping_message = DemoMessage::Ping(ping_count);
                        ping_count += 1;

                        match sink.send(ping_message).await {
                            Ok(_) => {
                                tracing::info!("Sent ping");
                            }
                            Err(error) => {
                                tracing::error!(?error, "Error sending ping");
                                break;
                            }
                        }
                    }
                    message = stream.next() => {
                        match message {
                            None => {
                                break;
                            }
                            Some(Ok(message)) => {
                                tracing::info!(?message, "Received message");

                                match message {
                                    DemoMessage::Ping(u) => {
                                        let response = DemoMessage::Pong(u);

                                        match sink.send(response).await {
                                            Ok(_) => {
                                                tracing::info!("Sent response");
                                            }
                                            Err(error) => {
                                                tracing::error!(?error, "Error sending response");
                                                break;
                                            }
                                        }
                                        tracing::info!("Received ping");
                                    }
                                    DemoMessage::Pong(_) => {
                                        tracing::info!("Received pong");
                                    }
                                    DemoMessage::Measurement(_) => {
                                        tracing::info!("Received measurement");
                                    }
                                }
                            }
                            Some(Err(error)) => {
                                tracing::error!(?error, "Error reading message");
                                break;
                            }
                        }
                    }
                }
            }

            tracing::debug!("Disconnected")
        });
    }
}
