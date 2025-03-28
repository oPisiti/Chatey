# Chatey

![chatey](https://github.com/user-attachments/assets/55049337-73b1-4b95-a861-55e347638850)

## Description
A webchat application based on websockets. Includes both server and client applications.

At startup, the client prompts for a username, which will then sign all of their messages.

Each message sent to and relayed from the server contains a timestamp, the user and the actual message, which are displayed in bubbles via the TUI.

The TUI also indicates entries and departures from the chatroom.

## Setup
Download rust, clone the repo and use :)

## Usage
### 0. Quick test (Recommended)
This repo provides a basic bash script (tmux.sh) which sets up multiple tmux panes.

Running it will spin up the server, along with two clients.

OR

### 1. Start the server
Run 
```bash 
cargo run -p server
```

This listens for incoming connections on port 5050.

### 2. Start the clients
Run 
```bash 
cargo run -p client
```

By default, it attempts to connection to a server on ```ws://127.0.0.1:5050```

You may change this by exporting the full path as the env variable SERVER_IP, i.e. ```SERVER_IP="ws://127.0.0.1:6060"```

## Logging
By default, the server logs directly to the terminal.

The clients, however, log to a file called "chatey_client.log"
