use std::{net::{IpAddr, Ipv4Addr, SocketAddr}, sync::Arc};

use futures_util::{
    stream::{SplitSink, SplitStream},
    SinkExt, StreamExt,
};
use shared::ChatMessage;
use simple_logger::SimpleLogger;
use time::macros::format_description;
use tokio::{
    net::TcpStream, select, sync::{mpsc::{unbounded_channel, UnboundedReceiver, UnboundedSender}, Mutex}
};
use tokio_tungstenite::{connect_async, tungstenite::Message, MaybeTlsStream, WebSocketStream};

mod tui;

// Aliases
type WSWrite = SplitSink<WebSocketStream<MaybeTlsStream<TcpStream>>, Message>;
type WSRead = SplitStream<WebSocketStream<MaybeTlsStream<TcpStream>>>;

// Global Constants
const LOCALHOST: IpAddr = IpAddr::V4(Ipv4Addr::new(127, 0, 0, 1));

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

    // Set server IP and port
    let url = match std::env::var("SERVER_IP") {
        Ok(value) => value,
        Err(_) => "ws://127.0.0.1:5050".to_string(),
    };

    // Set UI type: TEXT or TUI
    let tui: bool = std::env::var("TEXT").is_err();
    if tui{
        std::env::set_var("RUST_LOG", "off");
    }

    std::env::set_var("RUST_LOG", "off");

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

    // Utilities
    let history: Arc<Mutex<Vec<ChatMessage>>> = Arc::new(Mutex::new(Vec::new()));
    let (notifier_tx, notifier_rx ) = unbounded_channel();     
    let (input_tx, mut input_rx) = unbounded_channel();

    // TODO: Remove after tests
    for i in 0..4{
        history
            .lock()
            .await
            .push(ChatMessage::new(SocketAddr::new(LOCALHOST, 0), format!("{i}").into()));
    }

    // Init the TUI
    let history_clone = Arc::clone(&history);
    if tui{
        tokio::spawn(async {
            let terminal = ratatui::init();
            if let Err(run_error) = tui::run(terminal, history_clone,notifier_rx, input_tx).await{
                log::error!("Error while running TUI: {run_error}");
            };
            ratatui::restore();
        });
    }

    // Handle messages to and from the server
    let (mut ws_stream_write, mut ws_stream_read) = ws_stream.split();
    loop {
        select! {
            _ = handle_user_input(&mut input_rx, &mut ws_stream_write) => log::debug!("Message captured from user"),
            _ = handle_server_message(&mut ws_stream_read, Arc::clone(&history), notifier_tx.clone()) => log::debug!("Message received from server")
        }
    }
}

async fn handle_user_input(
    receiver: &mut UnboundedReceiver<String>,
    stream_write: &mut WSWrite,
) {
    // Wait for an input message from the TUI
    match receiver.recv().await{
        Some(input_string) => {
            let input_as_msg = Message::from(input_string);

            // Send message to server
            if let Err(err) = stream_write.send(input_as_msg.clone()).await {
                log::error!("Could not send message to server: {err}");
            }
        }
        None => log::error!("Input message from user not valid"),
    }
}

async fn handle_server_message(stream_read: &mut WSRead, history: Arc<Mutex<Vec<ChatMessage>>>, notifier_tx: UnboundedSender<()>) {
    match stream_read.next().await {
        Some(msg_result) => match msg_result {
            Ok(msg) => {
                let chat_msg = ChatMessage::new(SocketAddr::new(LOCALHOST, 0), msg.clone());
                history
                    .lock()
                    .await
                    .push(chat_msg);

                // Notify the TUI task of changes
                if let Err(notifier_error) = notifier_tx.send(()){
                    log::error!("Could not notify TUI task of new message from server: {notifier_error}");
                }
                
                log::info!("Received from server: {msg:?}");
            }
            Err(_) => log::error!("Received message from server is an error"),
        },
        None => log::error!("Could not read input stream from server properly"),
    }
}
