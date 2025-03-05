use std::{io::Error, sync::Arc};

use crossterm::event::{self, Event, KeyCode, KeyModifiers};
use futures_util::StreamExt;
use ratatui::{
    layout::{Constraint, Layout, Rect},
    style::Stylize,
    widgets::{Block, Paragraph, Wrap},
    DefaultTerminal,
};
use tokio::{
    select,
    sync::{mpsc::UnboundedReceiver, Mutex},
};
use tokio_tungstenite::tungstenite::Message;

/// Runs the TUI loop and prints the latest messages in 'history'
/// The loop awaits until a '()' notification is received via 'notify_rx'
/// The TUI will NOT be updated otherwise
pub async fn run(
    mut terminal: DefaultTerminal,
    history: Arc<Mutex<Vec<Message>>>,
    mut notify_rx: UnboundedReceiver<()>,
) {
    let mut keyboard_reader = event::EventStream::new();
    loop {
        // Config and draw the TUI
        let draw_result = terminal.draw(|frame| {
            let (area_title, layout) = calculate_layout(frame.area());
            let outer_block = Block::bordered().title("Outer block");
            let inner_block = Block::bordered().title("Inner block");

            let area = layout[0][0];
            let inner = outer_block.inner(area);
            frame.render_widget(outer_block, area);
            frame.render_widget(
                Paragraph::new("dawd".dark_gray()).wrap(Wrap { trim: true }),
                inner,
            );
        });

        // TODO: deal with draw result
        match draw_result{
            Ok(_) => {},
            Err(_) => {},
        }

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
                        _ => return
                    },
                    Some(Err(_)) => break,
                    None => break,
                }
            }

        }
    }


}

fn handle_keyboard_event(event: Option<Result<Event, Error>>) {}

/// Calculate the layout of the UI elements.
/// Returns a tuple of the title area and the main areas.
fn calculate_layout(area: Rect) -> (Rect, Vec<Vec<Rect>>) {
    let main_layout = Layout::vertical([Constraint::Length(1), Constraint::Min(0)]);
    let block_layout = Layout::vertical([Constraint::Max(4); 4]);
    let [title_area, main_area] = main_layout.areas(area);
    let main_areas = block_layout
        .split(main_area)
        .iter()
        .map(|&area| {
            Layout::horizontal([Constraint::Percentage(50), Constraint::Percentage(50)])
                .split(area)
                .to_vec()
        })
        .collect();
    (title_area, main_areas)
}
