use std::{fmt, time};

pub struct Message{
    from: String,
    to: String,
    timestamp: time::Instant,
    message: String
}
impl Message {
    pub fn new(from: String, to: String, message: String) -> Self{
        Self{
            from,
            to, 
            timestamp: time::Instant::now(),
            message
        }
    }
}
impl fmt::Display for Message {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "{} -> {}: {}, {} s ago", self.from, self.to, self.message, self.timestamp.elapsed().as_secs())

    }
}

pub enum HandleResult{
    ResponseSuccessful,
}

pub enum HandleError{
    ConnectionDropped,
    MalformedMessage
}

