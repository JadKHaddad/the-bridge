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

            while let Some(message) = stream.next().await {
                match message {
                    Ok(message) => {
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
                    Err(error) => {
                        tracing::error!(?error, "Error reading message");
                        break;
                    }
                }
            }

            tracing::debug!("Disconnected")
        });
    }
}
