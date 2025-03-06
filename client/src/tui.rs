use std::{char::MAX, fmt::Display, io::Error, sync::Arc};

use crossterm::event::{self, Event, KeyCode, KeyModifiers};
use futures_util::StreamExt;
use ratatui::{
    layout::{Constraint, Layout, Margin, Rect},
    style::{Color, Style, Stylize},
    widgets::{Block, Borders, Padding, Paragraph, Wrap},
    DefaultTerminal,
};
use shared::ChatMessage;
use tokio::{
    select,
    sync::{mpsc::UnboundedReceiver, Mutex},
};

/// Runs the TUI loop and prints the latest messages in 'history'
/// The loop awaits until a '()' notification is received via 'notify_rx'
/// The TUI will NOT be updated otherwise
pub async fn run(
    mut terminal: DefaultTerminal,
    history: Arc<Mutex<Vec<ChatMessage>>>,
    mut notify_rx: UnboundedReceiver<()>,
) -> Result<(), Error> {

    // Constants
    const MAX_MESSAGES_ON_SCREEN: u8 = 8;
    const PADDING: Padding = Padding::new(1, 1, 0, 0);

    let mut keyboard_reader = event::EventStream::new();
    loop {
        // Create outer block
        let outer_block = Block::default()
            .borders(Borders::ALL)
            .padding(PADDING)
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
                            .padding(PADDING)
                    )
                    .style(Style::default().fg(Color::White).bg(Color::Black))
            })
            .collect();

        // Draw a frame
        let draw_result = terminal.draw(|frame| {
            let outer = frame.area();

            // Create layouts
            let vertical = Layout::vertical([Constraint::Ratio(1, MAX_MESSAGES_ON_SCREEN.into()); MAX_MESSAGES_ON_SCREEN as usize]);
            let horizontal = Layout::horizontal([Constraint::Ratio(1, 3); 3]);

            // Creating areas
            let vertical_areas: [Rect; MAX_MESSAGES_ON_SCREEN as usize] = vertical.areas(outer.inner(Margin::new(1, 1)));

            // The final message areas are an array of [left, mid, right] areas
            // Each message area is meant to be used with a single of the three areas
            let mut msg_areas: Vec<[Rect; 3]> = Vec::new();
            for v in vertical_areas.iter().rev(){
                msg_areas.push(horizontal.areas(*v));
            }

            // Draw each widget
            frame.render_widget(outer_block, outer);
            for (i, msg) in msg_blocks.iter().enumerate(){
                frame.render_widget(msg, msg_areas[i][0]);
            }
        });

        // TODO: deal with draw result
        // match draw_result{
            // Ok(_) => {},
            // Err(_) => {},
        // }

        select! {
            // Wait for a change in history notification via "notify_rx"
            _ = notify_rx.recv() => todo!(),

            // TODO: Properly deal with all possibillities
            // Wait for a key to be pressed
            keyboard_event = keyboard_reader.next() => { 
                match keyboard_event{
                    Some(Ok(event)) => match event {
                        Event::Key(key) => if key.code == KeyCode::Esc { break }
                        else if key.modifiers == KeyModifiers::CONTROL && key.code == KeyCode::Char('c') {
                            break;
                        },
                        _ => continue,
                    },
                    Some(Err(_)) => break,
                    None => break,
                }
            },
        }
    }
    Ok(())
}

/// Calculate the layout of the UI elements.
/// Returns a tuple of the title area and the main areas.
fn calculate_layout(area: Rect) -> (Rect, Vec<Vec<Rect>>) {
    let main_layout = Layout::vertical([Constraint::Length(0), Constraint::Min(0)]);
    let block_layout = Layout::vertical([Constraint::Max(15); 10]);
    let [title_area, main_area] = main_layout.areas(area);
    let main_areas = block_layout
        .split(main_area)
        .iter()
        .map(|&area| {
            Layout::horizontal([Constraint::Percentage(100)])
                .split(area)
                .to_vec()
        })
        .collect();
    (title_area, main_areas)
}
