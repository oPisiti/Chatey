//********************************************************************
// Author: Lauro França (oPisiti)                                    #
// Contact:                                                          #
//   github: oPisiti                                                 #
//   Email: contact@opisiti.com                                      #
// Date: 2025                                                        #
//********************************************************************

use std::{collections::HashMap, net::SocketAddr, sync::Arc};

use futures_util::{stream::{SplitSink, SplitStream}, SinkExt, StreamExt};
use shared::{ChatMessage, ClientMessage, HandleError, HandleResult};
use tokio::{net::TcpStream, sync::{mpsc::{UnboundedReceiver, UnboundedSender}, Mutex}};
use tokio_tungstenite::{tungstenite::{Error, Message}, WebSocketStream};

pub type Tx = UnboundedSender<ChatMessage>;
pub type PeerMap = Arc<Mutex<HashMap<SocketAddr, Tx>>>;
pub type UsernameMap = Arc<Mutex<HashMap<SocketAddr, String>>>;

/// Closes a websocket stream that has been split into two
pub async fn close_websocket_stream(
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
pub async fn handle_received_from_client(
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

            // Broadcast exit of current user
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
pub async fn broadcast_message(message: ChatMessage, active_websockets: &PeerMap) {
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
pub async fn handle_received_from_server(
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
