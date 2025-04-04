//********************************************************************
// Author: Lauro França (oPisiti)                                    #
// Contact:                                                          #
//   github: oPisiti                                                 #
//   Email: contact@opisiti.com                                      #
// Date: 2025                                                        #
//********************************************************************

use crossterm::event::{Event, KeyCode, KeyModifiers, MouseEventKind};
use futures_util::{
    SinkExt, StreamExt,
};
use std::{io::Error, sync::Arc};
use shared::{ClientMessage, HandleError, WSRead, WSWrite};
use tokio::sync::{mpsc::{UnboundedReceiver, UnboundedSender}, Mutex};
use tokio_tungstenite::tungstenite::Message;


/// Custom enum for keyboard handling
pub enum HandlingSignal{
    Continue,
    End,
    Quit,
}

/// Awaits a message from receiver and attempts to relay it to the server
/// If the received message is None, returns a "HandleError::ConnectionDropped" error
pub async fn handle_user_input(
    receiver: &mut UnboundedReceiver<String>,
    stream_write: &mut WSWrite,
) -> Result<(), HandleError> {
    // Wait for an input message from the TUI
    match receiver.recv().await {
        Some(input_string) => {
            let input_as_msg = Message::from(input_string);

            // Send message to server
            if let Err(err) = stream_write.send(input_as_msg.clone()).await {
                log::error!("Could not send message to server: {err}");
            }
        }
        None => {
            log::error!("Receiving channel has been closed");
            return Err(HandleError::ConnectionDropped);
        }
    }
    Ok(())
}

/// Awaits for and deals with a message received from the server via "stream_read" and appends it as
/// a ClientMessage in "history"
/// Notifies the TUI for this new message, if valid
/// If the received message is None, returns a "HandleError::ConnectionDropped" error
pub async fn handle_server_message(
    stream_read: &mut WSRead,
    history: Arc<Mutex<Vec<ClientMessage>>>,
    notifier_tx: UnboundedSender<()>,
) -> Result<(), HandleError> {
    match stream_read.next().await {
        Some(msg_result) => match msg_result {
            Ok(msg) => match serde_json::from_str(msg.to_string().as_str()) {
                Ok(rec_msg) => {
                    // Append to history
                    history.lock().await.push(rec_msg);

                    // Notify the TUI task of changes
                    if let Err(notifier_error) = notifier_tx.send(()) {
                        log::error!("Could not notify TUI task of new message from server: {notifier_error}");
                    }

                    log::info!("Received from server: {msg:?}");
                }
                Err(err) => {
                    log::error!("Could not deserialize message from server: {err}");
                }
            },
            Err(_) => log::error!("Received message from server is an error"),
        },
        None => return Err(HandleError::ConnectionDropped),
    }

    Ok(())
}

/// Handles a single keyboard event and returns a signal
/// Will write char to buffer, as well as pop from it in case of Backspace input
pub fn handle_input_event(keyboard_event: Option<Result<Event, Error>>, buffer: &mut Vec<char>, scroll: &mut i8) -> HandlingSignal {
    match keyboard_event{
        Some(Ok(event)) => match event {
            Event::Key(key) => match key.code{
                KeyCode::Esc => return HandlingSignal::Quit,
                KeyCode::Char(char) =>{
                    if char == 'c' && key.modifiers == KeyModifiers::CONTROL {
                        return HandlingSignal::Quit
                    }

                    // Update input box
                    buffer.push(char);
                }
                KeyCode::Backspace => _ = buffer.pop(),
                KeyCode::Enter => {
                    return HandlingSignal::End;
                },
                _ => return HandlingSignal::Continue,
            }
            Event::Mouse(mouse) => match mouse.kind{
                MouseEventKind::ScrollDown => *scroll = -1,
                MouseEventKind::ScrollUp => *scroll = 1,
                _ => return HandlingSignal::Continue,
            }
            _ => return HandlingSignal::Continue,
        },
        Some(Err(_)) => return HandlingSignal::Quit,
        None => return HandlingSignal::Quit,
    }

    HandlingSignal::Continue
}
