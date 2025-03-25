//********************************************************************
// Author: Lauro França (oPisiti)                                    #
// Contact:                                                          #
//   github: oPisiti                                                 #
//   Email: contact@opisiti.com                                      #
// Date: 2025                                                        #
// Description:                                                      #
//   The main function for the chat server                           #
//********************************************************************

use futures_util::
    StreamExt
;
use helpers::*;
use shared::{ChatMessage, HandleError, HandleResult};
use simple_logger::SimpleLogger;
use std::{collections::HashMap, sync::Arc};
use time::macros::format_description;
use tokio::{
    io,
    net::TcpListener,
    select,
    sync::{
        mpsc::unbounded_channel,
        Mutex,
    },
};
use tokio_tungstenite::
    accept_async
;

mod helpers;

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
    let listener = TcpListener::bind("0.0.0.0:".to_string() + listening_port).await?;

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
