use std::{sync::Arc, time::Duration};

use futures_util::StreamExt;
use shared::ClientMessage;
use tokio::{
    select,
    sync::{mpsc::unbounded_channel, Mutex},
    time::sleep,
};
use tokio_tungstenite::connect_async;

mod handlers;
mod tui;

#[tokio::main]
async fn main() {
    // Set default logging level
    if std::env::var("RUST_LOG").is_err() {
        std::env::set_var("RUST_LOG", "info")
    }

    // Set server IP and port
    let url = match std::env::var("SERVER_IP") {
        Ok(value) => value,
        Err(_) => "ws://127.0.0.1:5050".to_string(),
    };

    // Init logger
    simple_logging::log_to_file("chatey_client.log", log::LevelFilter::Debug)
        .expect("Unable to set log to file");

    // Connection loop
    'outer: loop{
        // Attempt to connect to server
        let ws_stream = loop {
            if let Ok((ws_stream, _)) = connect_async(&url).await {
                break ws_stream;
            }
            println!("Failed to connect to server. Retrying in 5 s");
            sleep(Duration::from_secs(5)).await;
        };

        // Split the stream so it can be actually useful
        let (mut ws_stream_write, mut ws_stream_read) = ws_stream.split();

        // Utilities
        let history: Arc<Mutex<Vec<ClientMessage>>> = Arc::new(Mutex::new(Vec::new()));
        let (notifier_tx, notifier_rx) = unbounded_channel();
        let (input_tx, mut input_rx) = unbounded_channel();

        // Init the TUI
        let history_clone = Arc::clone(&history);
        let tui_handler = tokio::spawn(async {
            let terminal = ratatui::init();
            if let Err(run_error) =
                tui::run_chat(terminal, history_clone, notifier_rx, input_tx).await
            {
                log::error!("Error while running TUI: {run_error}");
            };
            ratatui::restore();
        });

        // Handle messages to and from the server
        loop {
            select! {
                handle_result = handlers::handle_user_input(&mut input_rx, &mut ws_stream_write) => match handle_result{
                    Ok(_) => log::debug!("Message captured from user"),
                    Err(_) => {
                        tui_handler.abort();
                        ratatui::restore();
                        continue 'outer;
                    },
                },
                handle_result = handlers::handle_server_message(&mut ws_stream_read, Arc::clone(&history), notifier_tx.clone()) => match handle_result{
                    Ok(_) => log::debug!("Message received from server"),
                    Err(_) => {
                        tui_handler.abort();
                        ratatui::restore();
                        continue 'outer;
                    },
                }
            }
        }
    }
}
