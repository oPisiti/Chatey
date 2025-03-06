use std::{fmt, net::SocketAddr, time};
use tokio_tungstenite::tungstenite::Message;

#[derive(Clone)]
pub struct ChatMessage{
    from: SocketAddr,
    timestamp: time::Instant,
    message: Message
}
impl ChatMessage {
    pub fn new(from: SocketAddr, message: Message) -> Self{
        Self{
            from,
            timestamp: time::Instant::now(),
            message
        }
    }

    pub fn get_message(&self) -> Message{
        self.message.clone()
    }

    pub fn get_addr(&self) -> SocketAddr{
        self.from
    }
}
impl fmt::Display for ChatMessage {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "{} : {}, {} s ago", self.from, self.message.to_text().unwrap_or_default(), self.timestamp.elapsed().as_secs())
    }
}

pub enum HandleResult{
    ResponseSuccessful,
}

pub enum HandleError{
    ConnectionDropped,
    MalformedMessage
}

