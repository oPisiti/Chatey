use futures_util::{
    stream::{SplitSink, SplitStream},
    SinkExt, StreamExt,
};
use simple_logger::SimpleLogger;
use std::{collections::HashMap, net::SocketAddr, sync::Arc};
use time::macros::format_description;
use tokio::{
    io,
    net::{TcpListener, TcpStream},
    select,
    sync::{
        mpsc::{unbounded_channel, UnboundedReceiver, UnboundedSender},
        Mutex,
    },
};
use tokio_tungstenite::{accept_async, tungstenite::Message, WebSocketStream};

type Tx = UnboundedSender<Message>;
type PeerMap = Arc<Mutex<HashMap<SocketAddr, Tx>>>;

#[tokio::main]
async fn main() -> io::Result<()> {
    // Set default logging level
    if std::env::var("RUST_LOG").is_err() {
        std::env::set_var("RUST_LOG", "debug")
    }

    println!(
        "Logging level set to {:?}",
        std::env::var("RUST_LOG").unwrap()
    );

    // Init logger
    SimpleLogger::new()
        .with_level(log::LevelFilter::Trace)
        .env()
        .with_timestamp_format(format_description!(
            "[year]-[month]-[day] [hour]:[minute]:[second]"
        ))
        .init()
        .unwrap();

    // The main task will handle listening
    let listening_port = "5050";
    let listener = TcpListener::bind("127.0.0.1:".to_string() + listening_port).await?;

    log::info!("Listening for incoming connections on port {listening_port}");

    // Listen for connections and try to upgrade to websocket
    let active_websockets: PeerMap = Arc::new(Mutex::new(HashMap::new()));
    while let Ok((stream, ip)) = listener.accept().await {
        log::info!("Accepted a tcp connection from {ip}. Attempting to upgrade to WebSocket...");

        let ws_stream = match accept_async(stream).await {
            Ok(result) => result,
            Err(err) => {
                log::error!("Could not upgrade connection of ip {ip}: {err}");
                continue;
            }
        };

        log::info!("Connection upgraded successfully");

        // Handle each connection on a separate task
        let cloned_active_websockets = Arc::clone(&active_websockets);
        tokio::spawn(async move {
            // Add websocket to active
            let (tx, mut rx) = unbounded_channel();
            cloned_active_websockets.lock().await.insert(ip, tx.clone());

            // WebSocketStream must be split in order to be useful for IO
            let (mut write, mut read) = ws_stream.split();

            // Keep listening for messages from client or from server
            loop {
                // Select between receiveing from the server and broadcasting messages received from the websocket
                select! {
                        _ = handle_received_from_client(&cloned_active_websockets, &mut read) => log::debug!("Message received from client"),
                        _ = handle_received_from_server(&mut rx, &mut write) => log::debug!("Message retransmitted to client"),
                }
            }
        });
    }

    Ok(())
}

/// Waits for a message from the client and then broadcasts it to all the other
/// connected piers.
async fn handle_received_from_client(
    active_websockets: &PeerMap,
    stream_read: &mut SplitStream<WebSocketStream<TcpStream>>,
) {

    match stream_read.next().await {
        Some(message_result) => {
            if let Ok(message) = message_result {
                broadcast_message(message, active_websockets).await;
            }
        }
        None => {
            log::error!("Could not handle message received from client");
        }
    }
}

/// Broadcasts a message to all connectec websockets in 'active_websockets'
async fn broadcast_message(message: Message, active_websockets: &PeerMap) {
    let mut inactive_addrs: Vec<SocketAddr> = Vec::new();

    // Broadcasts a message to all clients connected in active_websockets
    let mut actives = active_websockets.lock().await;
    for (addr, sender) in actives.iter() {
        if let Err(send_error) = sender.send(message.clone()) {
            log::error!("Could not broadcast message to {addr}: {send_error}");
            inactive_addrs.push(*addr);
        }
    }

    // Delete current channel from active sockets
    for inactive in inactive_addrs {
        log::debug!("Removing inactive channel for addr {inactive}");
        actives.remove(&inactive);
    }
}

/// Relays message to specified client
async fn handle_received_from_server(
    rx: &mut UnboundedReceiver<Message>,
    write: &mut SplitSink<WebSocketStream<TcpStream>, Message>,
) {
    match rx.recv().await {
        Some(message) => {
            if write.send(message).await.is_err() {
                log::error!("Could not send message back to client");
            }
        }
        None => log::error!("Nothing came back from recv :("),
    };
}
