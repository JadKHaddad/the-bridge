use bincode_bridge::{
    decode::framed_read::FramedRead, encode::framed_write::FramedWrite, tokio::Compat, Message,
};

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
            let read_buf: &mut [u8] = &mut [0; 23];
            let write_buf: &mut [u8] = &mut [0; 100];

            let tokio_read_compat = Compat::new(reader);
            let tokio_write_compat = Compat::new(writer);

            let mut reader: FramedRead<'_, _, Message> =
                FramedRead::new(tokio_read_compat, read_buf);
            let mut writer: FramedWrite<'_, _, Message> =
                FramedWrite::new(tokio_write_compat, write_buf);

            loop {
                let message = reader.read_frame().await;
                match message {
                    Ok(message) => {
                        tracing::info!(?message, "Received message");

                        let response = Message::C(
                            100, 100, 100, 100, 100, 100, 100, 100, 100, 100, 100, 100, 100, 100,
                            100, 100, 100,
                        );

                        match writer.write_frame(&response).await {
                            Ok(_) => {
                                tracing::info!("Sent response");
                            }
                            Err(error) => {
                                tracing::error!(?error, "Error sending response");
                                break;
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
