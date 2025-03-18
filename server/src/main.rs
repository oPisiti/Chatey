use futures_util::{
    stream::{SplitSink, SplitStream},
    SinkExt, StreamExt,
};
use shared::{ChatMessage, ClientMessage, HandleError, HandleResult};
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
use tokio_tungstenite::{
    accept_async,
    tungstenite::{Error, Message},
    WebSocketStream,
};

type Tx = UnboundedSender<ChatMessage>;
type PeerMap = Arc<Mutex<HashMap<SocketAddr, Tx>>>;
type UsernameMap = Arc<Mutex<HashMap<SocketAddr, String>>>;

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
    let connection_to_username: UsernameMap = Arc::new(Mutex::new(HashMap::new()));
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
        let cloned_con_to_username = Arc::clone(&connection_to_username);
        tokio::spawn(async move {
            // Add websocket to active
            let (tx, mut rx) = unbounded_channel();
            cloned_active_websockets.lock().await.insert(ip, tx.clone());

            // Expect a message which should contain the username
            let (mut write, mut read) = ws_stream.split();
            let username = match read.next().await {
                Some(name_result) => match name_result {
                    Ok(name) => name.to_string(),
                    Err(err) => {
                        log::error!("Invalid username message: {err}. Closing connection");
                        if close_websocket_stream(write, read).await.is_err() {
                            log::error!("Could not close connection. Aborting connection");
                        };
                        return;
                    }
                },
                None => {
                    log::error!("Invalid username message. Closing connection");
                    if close_websocket_stream(write, read).await.is_err() {
                        log::error!("Could not close connection. Aborting all");
                    };
                    return;
                }
            };

            // Save the username in the hashmap
            cloned_con_to_username.lock().await.insert(ip, username.clone());

            // Broadcast arrival of current user
            match ChatMessage::build(ip, "SYSTEM".to_string(), format!("{username} has entered the channel")){
                Some(entry_message) => broadcast_message(entry_message, &cloned_active_websockets).await,
                None => log::error!("Could not create user entry broadcast message"),
            }

            // Keep listening for messages from client or from server
            loop {
                // Select between receiveing from the server and broadcasting messages received from the websocket
                select! {
                    handle_result = handle_received_from_client(&cloned_active_websockets, &cloned_con_to_username, &mut read, ip) => {
                        match handle_result{
                            Ok(HandleResult::ResponseSuccessful) => log::debug!("Response successfully sent to {} ({ip})", cloned_con_to_username.lock().await.get(&ip).unwrap_or(&"Unknown".to_string())),
                            Err(HandleError::MalformedMessage) => log::debug!("Malformed message received from client {ip}. Ignoring"),
                            Err(HandleError::ConnectionDropped) => {
                                log::debug!("Connection with client {ip} interrupted.");
                                return;
                            },
                            Err(HandleError::UnkownClient) => log::error!("Unkown client"),
                        }
                    },
                    handle_result = handle_received_from_server(&mut rx, &mut write) => match handle_result {
                        Ok(HandleResult::ResponseSuccessful) => log::debug!("Response successfully sent to {} ({ip})", cloned_con_to_username.lock().await.get(&ip).unwrap_or(&"Unknown".to_string())),
                        Err(HandleError::MalformedMessage) => log::debug!("Malformed message received from client {ip}. Ignoring"),
                        Err(HandleError::ConnectionDropped) => {
                            log::debug!("Connection with client {ip} interrupted.");
                            return;
                        },
                        Err(HandleError::UnkownClient) => log::error!("Unkown client"),
                    }
                }
            }
        });
    }

    Ok(())
}

/// Closes a websocket stream that has been split into two
async fn close_websocket_stream(
    mut write: SplitSink<WebSocketStream<TcpStream>, Message>,
    mut read: SplitStream<WebSocketStream<TcpStream>>,
) -> Result<(), Error> {
    // Send a close message
    write.send(Message::Close(None)).await?;

    // Keep pulling from read stream until nothing more is left
    while let Some(msg) = read.next().await {
        match msg {
            Ok(msg) => {
                if msg.is_close() {
                    return Ok(());
                }
            }
            Err(e) => return Err(e),
        }
    }
    Ok(())
}

/// Waits for a message from the client and then broadcasts it to all the other
/// connected piers.
async fn handle_received_from_client(
    active_websockets: &PeerMap,
    con_to_username: &UsernameMap,
    stream_read: &mut SplitStream<WebSocketStream<TcpStream>>,
    client_addr: SocketAddr,
) -> Result<HandleResult, HandleError> {

    let username = con_to_username
        .lock()
        .await
        .get(&client_addr)
        .ok_or(HandleError::UnkownClient)?
        .to_owned();

    match stream_read.next().await {
        Some(message_result) => {
            if let Ok(message) = message_result {
                // Wrap the tungstenite message in a ChatMessage
                let chat_message = ChatMessage::build(client_addr, username, message.to_string())
                    .ok_or(HandleError::MalformedMessage)?;

                broadcast_message(chat_message, active_websockets).await;
                return Ok(HandleResult::ResponseSuccessful);
            }

            Err(HandleError::MalformedMessage)
        }
        None => {
            log::info!("Client connection returned None. Removing client from connected peers");
            let mut sockets = active_websockets.lock().await;
            sockets.remove(&client_addr);

            // Broadcast exit of current user
            // TODO: Fix exit message not being broadcast
            match ChatMessage::build(client_addr, "SYSTEM".to_string(), format!("{username} has exited the channel")){
                Some(exit_message) => {
                    log::info!("Broadcasting {username}'s exit message");
                    broadcast_message(exit_message, active_websockets).await;
                },
                None => log::error!("Could not create user {username}'s exit broadcast message"),
            }

            Err(HandleError::ConnectionDropped)
        }
    }
}

/// Broadcasts a message to all connected websockets in 'active_websockets'
async fn broadcast_message(message: ChatMessage, active_websockets: &PeerMap) {
    let mut inactive_addrs: Vec<SocketAddr> = Vec::new();

    // Broadcasts a message to all clients connected in active_websockets
    let mut actives = active_websockets.lock().await;

    for (addr, sender) in actives.iter() {
        if *addr == message.get_addr() {
            continue;
        }

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
    rx: &mut UnboundedReceiver<ChatMessage>,
    write: &mut SplitSink<WebSocketStream<TcpStream>, Message>
) -> Result<HandleResult, HandleError> {
    match rx.recv().await {
        Some(message) => {
            // Create a ClientMessage
            let client_msg = ClientMessage::from(message);

            // Serialize and send
            match serde_json::to_string(&client_msg) {
                Ok(ser_msg) => {
                    if write.send(Message::Text(ser_msg.into())).await.is_err() {
                        log::error!("Could not send message back to client");
                        return Err(HandleError::ConnectionDropped);
                    }

                    Ok(HandleResult::ResponseSuccessful)
                }
                Err(err) => {
                    log::error!("Could not serialize message to be relayed to client: {err}");
                    Err(HandleError::MalformedMessage)
                }
            }
        }
        None => {
            log::error!("Nothing came back from recv :(");
            Err(HandleError::MalformedMessage)
        },
    }
}
