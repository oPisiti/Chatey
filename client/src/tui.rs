use std::{io::Error, sync::Arc};

use crossterm::event::{self, Event, KeyCode, KeyModifiers};
use futures_util::StreamExt;
use ratatui::{
    layout::{Constraint, Flex, Layout, Margin, Rect}, style::{Color, Style}, text::Line, widgets::{Block, BorderType, Borders, Padding, Paragraph}, DefaultTerminal
};
use shared::ClientMessage;
use tokio::{
    select,
    sync::{mpsc::{UnboundedReceiver, UnboundedSender}, Mutex},
};

// Constants
const MAX_MESSAGES_ON_SCREEN: u8 = 8;
const PADDING_INSIDE: Padding = Padding::new(1, 1, 0, 0);
const CURSOR_CHAR: &str = "_";
const CLIENT_USERNAME: &str = "You";
const SYSTEM_USERNAME: &str = "SYSTEM";

/// Custom enum for keyboard handling
enum HandlingSignal{
    Continue,
    End,
    Quit,
}

/// Runs the TUI loop and prints the latest messages in 'history'
/// The loop awaits until a '()' notification is received via 'notify_rx'
/// The TUI will NOT be updated otherwise
pub async fn run_chat(
    terminal: DefaultTerminal,
    history: Arc<Mutex<Vec<ClientMessage>>>,
    mut notifier_rx: UnboundedReceiver<()>,
    input_tx: UnboundedSender<String>
) -> Result<(), Error> {

    let mut input_box = Vec::new();
    let mut username = Vec::new();
    let mut keyboard_reader = event::EventStream::new();
    let mut terminal = terminal;

    // Create layouts
    let username_vert_layout = Layout::vertical([
        Constraint::Percentage(45),
        Constraint::Fill(1),
        Constraint::Percentage(45)
    ]);
    let username_horizontal_layout = Layout::horizontal([
        Constraint::Percentage(50)
    ])
        .flex(Flex::Center);
    let msg_input_layout = Layout::vertical([
        Constraint::Percentage(90),
        Constraint::Fill(1),
    ]);
    let msg_vertical_layout = Layout::vertical([
        Constraint::Ratio(1, MAX_MESSAGES_ON_SCREEN.into());
        MAX_MESSAGES_ON_SCREEN as usize
    ]);
    let msg_horizontal_layout = Layout::horizontal([
        Constraint::Percentage(35),
        Constraint::Fill(1),
        Constraint::Percentage(35),
    ]);

    // Prompt the user for a username
    loop{
        let username_block = Paragraph::new(username.iter().collect::<String>() + CURSOR_CHAR)
            .block(Block::bordered()
                .padding(PADDING_INSIDE)
                .title_top(Line::from("Set a username").centered())
            )
            .style(Style::default().fg(Color::White).bg(Color::Black));

        let draw_result = terminal.draw(|frame|{
            let [_, username_vert_area, _] = username_vert_layout.areas(frame.area().inner(Margin::new(1, 1)));
            let [username_area] = username_horizontal_layout.areas(username_vert_area);
            frame.render_widget(username_block, username_area);
        });

        // Deal with draw result
        if draw_result.is_err() {
            log::error!("Failed to render frame: {}", draw_result.unwrap_err());
        }
        
        // Handle input
        match handle_keyboard_event(keyboard_reader.next().await, &mut username){
            HandlingSignal::Continue => continue,
            HandlingSignal::End => break,
            HandlingSignal::Quit => return Err(std::io::Error::other("")),
        }
    }

    let username_string = username.iter().collect::<String>();
    if input_tx.send(username_string.clone()).is_err(){
        log::error!("Could not send username message back to main");
        return Err(std::io::Error::other(""))
    };

    // Main chat loop
    let chat_title = format!("Logged in as {username_string}");
    loop {
        // Create outer block
        let outer_block = Block::bordered()
            .padding(PADDING_INSIDE)
            .style(Style::default().fg(Color::White).bg(Color::Black))
            .title_top(Line::from(chat_title.clone()).centered());

        // Create message blocks
        let msg_blocks: Vec<(Paragraph, usize)> = history
            .lock()
            .await
            .iter()
            .rev()
            .take(MAX_MESSAGES_ON_SCREEN as usize)
            .map(|client_message| {
                let position_index: usize = match client_message.get_username().as_str(){
                    CLIENT_USERNAME => 2,
                    SYSTEM_USERNAME => 1,
                    _ => 0
                };

                // Define the message title (at the bottom of the paragraph)
                let mut title = Line::from(client_message.get_metadata());
                if position_index == 0 {title = title.left_aligned()}
                else if position_index == 1 {title = title.centered()}
                else if position_index == 2 {title = title.right_aligned()}

                // Define the paragraph
                let mut parag = Paragraph::new(client_message.get_message().to_owned())
                    .block(Block::bordered()
                        .title_bottom(title)
                        .padding(PADDING_INSIDE)
                        .border_type(BorderType::Rounded),
                    )
                    .style(Style::default().fg(Color::White).bg(Color::Black));

                if position_index == 2 {parag = parag.right_aligned()} 

                (parag, position_index)
            })
            .collect();

        // Create the input block
        let input_string = input_box.iter().collect::<String>() + CURSOR_CHAR;
        let input_block = Paragraph::new(input_string)
            .block(
                Block::default()
                    .borders(Borders::TOP)
                    .padding(PADDING_INSIDE),
            )
            .style(Style::default().fg(Color::White).bg(Color::Black));

        // Draw a frame
        let draw_result = terminal.draw(|frame| {
            // --- Creating areas ---
            // Outer frame
            let outer = frame.area();

            // Devide outer into a messages box and an input box
            let [msg_area, input_area] = msg_input_layout.areas(outer.inner(Margin::new(1, 1)));

            // Devide the messages box into vertival parts
            let vertical_areas: [Rect; MAX_MESSAGES_ON_SCREEN as usize] =
                msg_vertical_layout.areas(msg_area);

            // The final message areas are an array of [left, mid, right] areas
            // Each message area is meant to be used with a single of the three areas
            let mut msg_areas: Vec<[Rect; 3]> = Vec::new();
            for v in vertical_areas.iter().rev() {
                msg_areas.push(msg_horizontal_layout.areas(*v));
            }

            // Draw each widget
            frame.render_widget(&outer_block, outer);
            frame.render_widget(input_block, input_area);
            for (i, (msg, index)) in msg_blocks.iter().enumerate() {
                frame.render_widget(msg, msg_areas[i][*index]);
            }
        });

        // Deal with draw result
        if draw_result.is_err() {
            log::error!("Failed to render frame: {}", draw_result.unwrap_err());
        }

        // Wait for an event to trigger a new TUI frame
        select! {
            // Wait for a change in history notification via "notify_rx"
            _ = notifier_rx.recv() => continue,

            // Wait for a key to be pressed
            keyboard_event = keyboard_reader.next() => match handle_keyboard_event(keyboard_event, &mut input_box){
                HandlingSignal::Continue => continue,
                HandlingSignal::End => {
                    let input_string: String = input_box.iter().collect();
                    if input_tx.send(input_string.clone()).is_err(){
                        log::error!("Could not send input message back to main")
                    };
                    
                    // Add input to history and clear input box
                    history.lock().await.push(
                        ClientMessage::new("You".to_string(), input_string)
                    );
                    input_box.clear();
                },
                HandlingSignal::Quit => return Err(std::io::Error::other("")),
            }
        }
    }
}

/// Handles a single keyboard event and returns a signal
/// Will write char to buffer, as well as pop from it in case of Backspace input
fn handle_keyboard_event(keyboard_event: Option<Result<Event, Error>>, buffer: &mut Vec<char>) -> HandlingSignal {
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
            _ => return HandlingSignal::Continue,
        },
        Some(Err(_)) => return HandlingSignal::Quit,
        None => return HandlingSignal::Quit,
    }

    HandlingSignal::Continue
}
