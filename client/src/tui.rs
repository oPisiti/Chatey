use std::{io::Error, iter::once, sync::Arc};

use crossterm::event::{self, Event, KeyCode, KeyModifiers};
use futures_util::StreamExt;
use ratatui::{
    layout::{Constraint, Layout, Margin, Rect},
    style::{Color, Style},
    widgets::{Block, Borders, Padding, Paragraph},
    DefaultTerminal,
};
use shared::ChatMessage;
use tokio::{
    select,
    sync::{mpsc::UnboundedReceiver, Mutex},
};

// Constants
const MAX_MESSAGES_ON_SCREEN: u8 = 8;
const PADDING_INSIDE: Padding = Padding::new(1, 1, 0, 0);
const CURSOR_CHAR: &str = "_";

/// Runs the TUI loop and prints the latest messages in 'history'
/// The loop awaits until a '()' notification is received via 'notify_rx'
/// The TUI will NOT be updated otherwise
pub async fn run(
    terminal: DefaultTerminal,
    history: Arc<Mutex<Vec<ChatMessage>>>,
    mut notify_rx: UnboundedReceiver<()>,
) -> Result<(), Error> {
    let mut input_box = Vec::new();
    let mut keyboard_reader = event::EventStream::new();
    let mut terminal = terminal;

    loop {
        // Create outer block
        let outer_block = Block::default()
            .borders(Borders::ALL)
            .padding(PADDING_INSIDE)
            .style(Style::default().fg(Color::White).bg(Color::Black));

        // Create message blocks
        let msg_blocks: Vec<Paragraph> = history
            .lock()
            .await
            .iter()
            .rev()
            .take(MAX_MESSAGES_ON_SCREEN as usize)
            .map(|chat_message| {
                Paragraph::new(format!("{chat_message}"))
                    .block(
                        Block::default()
                            .borders(Borders::ALL)
                            .padding(PADDING_INSIDE),
                    )
                    .style(Style::default().fg(Color::White).bg(Color::Black))
            })
            .collect();

        // Create the input block
        let mut input_string: String = input_box.iter().collect();
        input_string += CURSOR_CHAR;
        let input_block = Paragraph::new(input_string)
            .block(
                Block::default()
                    .borders(Borders::ALL)
                    .padding(PADDING_INSIDE),
            )
            .style(Style::default().fg(Color::White).bg(Color::Black));

        // Draw a frame
        let draw_result = terminal.draw(|frame| {
            let outer = frame.area();

            // Create layouts
            let vertical = Layout::vertical(
                [Constraint::Ratio(1, (MAX_MESSAGES_ON_SCREEN + 1).into());
                (MAX_MESSAGES_ON_SCREEN + 1) as usize],
            );
            let horizontal = Layout::horizontal([
                Constraint::Percentage(40),
                Constraint::Fill(1),
                Constraint::Percentage(40),
            ]);

            // Creating areas
            let vertical_areas: [Rect; (MAX_MESSAGES_ON_SCREEN + 1) as usize] =
                vertical.areas(outer.inner(Margin::new(1, 1)));

            // The final message areas are an array of [left, mid, right] areas
            // Each message area is meant to be used with a single of the three areas
            let mut msg_areas: Vec<[Rect; 3]> = Vec::new();
            for v in vertical_areas.iter().rev() {
                msg_areas.push(horizontal.areas(*v));
            }

            // Draw each widget
            frame.render_widget(&outer_block, outer);

            let blocks_iterator = once(&input_block).chain(msg_blocks.iter());
            for (i, msg) in blocks_iterator.enumerate() {
                frame.render_widget(msg, msg_areas[i][0]);
            }
        });

        // TODO: deal with draw result
        if draw_result.is_err() {
            log::error!("Failed to render frame: {}", draw_result.unwrap_err());
        }

        // Wait for an event to trigger a new TUI frame
        select! {
            // Wait for a change in history notification via "notify_rx"
            _ = notify_rx.recv() => todo!(),

            // TODO: Properly deal with all possibillities
            // Wait for a key to be pressed
            keyboard_event = keyboard_reader.next() => match keyboard_event{
                Some(Ok(event)) => match event {
                    Event::Key(key) => match key.code{
                        KeyCode::Esc => break,
                        KeyCode::Char(char) =>{
                            if char == 'c' && key.modifiers == KeyModifiers::CONTROL {break;}

                            // Update input box
                            input_box.push(char);
                        }
                        KeyCode::Backspace => _ = input_box.pop(),
                        _ => continue,
                    }
                    _ => continue,
                },
                Some(Err(_)) => break,
                None => break,
            },
        }
    }
    Ok(())
}
