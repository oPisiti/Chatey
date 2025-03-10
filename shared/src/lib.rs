use std::{fmt, net::SocketAddr, time::Instant};

use serde::{Deserialize, Serialize};

#[derive(Clone)]
pub struct ChatMessage{
    from_addr: SocketAddr,
    from_username: String,
    timestamp: Instant,
    message: String
}
impl ChatMessage {
    pub fn build(socket: SocketAddr, username: String, message: String) -> Option<Self>{
        Some(Self{
            from_addr: socket,
            from_username: username,
            timestamp: Instant::now(),
            message
        })
    }
    
    /// A getter method for the socket address
    pub fn get_addr(&self) -> SocketAddr{
        self.from_addr
    }

    /// A getter method for the username
    pub fn get_username(&self) -> String{
        self.from_username.clone()
    }

    /// A getter method for the message
    pub fn get_message(&self) -> String{
        self.message.clone()
    }

    /// Creates a client ChatMessage from a ClientMessage, overriding
    /// the timestamp and username (based on SocketAddr)
    pub fn from(msg: ClientMessage, from_addr: SocketAddr, from_username: String) -> Self {
        Self{
            timestamp: Instant::now(),
            from_addr,
            from_username,
            message: msg.input_message,
        }
    }
}
impl fmt::Display for ChatMessage {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "{} : {}, {} s ago", self.from_addr, self.message, self.timestamp.elapsed().as_secs())
    }
}

/// Created when the user finished inputting a message
#[derive(Debug, Serialize, Deserialize)]
pub struct ClientMessage{
    input_message: String,
    from_username: String,

    #[serde(with = "serde_millis")]
    timestamp: Instant,
}
impl ClientMessage{
    pub fn new(from_username: String, input_message: String) -> Self{
        Self{
            input_message,
            from_username,
            timestamp: Instant::now()
        }
    }

    /// A getter method for the message
    pub fn get_message(&self) -> String{
        self.input_message.clone()
    }

    /// A getter method for the username
    pub fn get_username(&self) -> String{
        self.from_username.clone()
    }

    /// Creates a ClientMessage from a ChatMessage
    pub fn from(input: ChatMessage) -> Self{
        Self::new(input.get_username(), input.get_message())
    }
}

pub enum HandleResult{
    ResponseSuccessful,
}

pub enum HandleError{
    ConnectionDropped,
    MalformedMessage,
    UnkownClient
}

