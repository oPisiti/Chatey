# Chatey

![chatey](https://github.com/user-attachments/assets/55049337-73b1-4b95-a861-55e347638850)


## Description
A webchat application using websockets.

Includes both server and client applications

## Setup

## Usage
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
