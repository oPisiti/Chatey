use futures_util::{
    stream::{SplitSink, SplitStream},
    SinkExt, StreamExt,
};
use simple_logger::SimpleLogger;
use time::macros::format_description;
use tokio::{io::stdin, net::TcpStream, select};
use tokio_tungstenite::{connect_async, tungstenite::Message, MaybeTlsStream, WebSocketStream};
use tokio_util::codec::{FramedRead, LinesCodec};

type WSWrite = SplitSink<WebSocketStream<MaybeTlsStream<TcpStream>>, Message>;
type WSRead = SplitStream<WebSocketStream<MaybeTlsStream<TcpStream>>>;

#[tokio::main]
async fn main() {
    // Set default logging level
    if std::env::var("RUST_LOG").is_err() {
        std::env::set_var("RUST_LOG", "info")
    }

    println!(
        "Logging level set to {:?}",
        std::env::var("RUST_LOG").unwrap()
    );

    let url = match std::env::var("SERVE_IP") {
        Ok(value) => value,
        Err(_) => "ws://127.0.0.1:5050".to_string(),
    };

    // Attempt to connect to server
    let (ws_stream, _) = connect_async(url)
        .await
        .expect("Failed to connect to server");

    // Init logger
    SimpleLogger::new()
        .with_level(log::LevelFilter::Trace)
        .env()
        .with_timestamp_format(format_description!(
            "[year]-[month]-[day] [hour]:[minute]:[second]"
        ))
        .init()
        .unwrap();

    let stdin = stdin();
    let mut stdin_reader = FramedRead::new(stdin, LinesCodec::new());
    let mut history: Vec<Message> = Vec::new();
    let (mut write, mut read) = ws_stream.split();
    loop {
        select! {
            _ = handle_message_from_user(&mut stdin_reader, &mut write) => log::debug!("Message captured from user"),
            _ = handle_message_from_server(&mut read, &mut history) => log::debug!("Message received from server")
        }
    }

}

async fn handle_message_from_user(
    reader: &mut FramedRead<tokio::io::Stdin, LinesCodec>,
    stream_write: &mut WSWrite,
) {
    // Listen for input form the terminal (user message)
    if let Some(input_result) = reader.next().await {
        // Try to send message to server
        match input_result {
            Ok(input_message) => {
                if let Err(err) = stream_write.send(input_message.into()).await {
                    log::error!("Could not send message to server: {err}");
                }
            }
            Err(_) => log::error!("Input message from user not valid"),
        }
    };
}

async fn handle_message_from_server(stream_read: &mut WSRead, history: &mut Vec<Message>) {
    match stream_read.next().await {
        Some(msg_result) => match msg_result {
            Ok(msg) => {
                history.push(msg.clone());
                log::info!("Received from server: {msg:?}");
            }
            Err(_) => log::error!("Received message from server is an error"),
        },
        None => log::error!("Could not read input stream from server properly"),
    }
}
