use futures_util::{stream::{SplitSink, SplitStream}, SinkExt, StreamExt};
use std::{collections::HashMap, net::SocketAddr, sync::Arc};
use simple_logger::SimpleLogger;
use time::macros::format_description;
use tokio::{io, net::{TcpListener, TcpStream}, select, sync::{mpsc::{unbounded_channel, UnboundedReceiver, UnboundedSender}, Mutex}};
use tokio_tungstenite::{accept_async, tungstenite::Message, WebSocketStream};

type Tx = UnboundedSender<Message>;
type PeerMap = Arc<Mutex<HashMap<SocketAddr, Tx>>>;

#[tokio::main]
async fn main() -> io::Result<()> {
    // Init logger
    SimpleLogger::new()
        .with_level(log::LevelFilter::Trace)
        .env()
        .with_timestamp_format(format_description!("[year]-[month]-[day] [hour]:[minute]:[second]"))
        .init()
        .unwrap();

    // The main task will handle listening 
    let listening_port = "5050";
    let listener = TcpListener::bind("127.0.0.1:".to_string() + listening_port)
        .await?;

    log::info!("Listening for incoming connections on port {listening_port}");

    // Listen for connections and try to upgrade to websocket
    let active_websockets: PeerMap = Arc::new(Mutex::new(HashMap::new()));
    while let Ok((stream, ip)) = listener.accept().await{
        log::info!("Accepted a tcp connection from {ip}. Attempting to upgrade to WebSocket...");

        let ws_stream = match accept_async(stream).await{
            Ok(result) => result,
            Err(_) => {
                log::error!("Could not upgrade connection of ip {ip}");
                continue
            }
        };

        let cloned_active_websockets = Arc::clone(&active_websockets);
        tokio::spawn(async {
            // Add websocket to active
            let (tx, rx) = unbounded_channel();

            let (write, read) = ws_stream.split();

            // Select between receiveing from the server and broadcasting messages received from the websocket
            select! {
                _ = handle_received_from_client(cloned_active_websockets, read) => log::debug!("Message received from client"),
                _ = handle_received_from_server(rx, write) => log::debug!("Message retransmitted to client"),
            }

            // handle_connection(Arc::clone(&active_websockets), ws_stream, ip)
        });

    }


    let mes = shared::Message::new(
        "a".to_string(),
        "b".to_string(),
        "blablabla".to_string()
    );
    println!("{mes}");

    println!("ad");
    
    Ok(())
}

async fn handle_connection(active_ws_mutex: PeerMap, ws_stream: WebSocketStream<TcpStream>, ip: SocketAddr){
    // Add websocket to active
    let (tx, rx) = unbounded_channel();

    let (write, read) = ws_stream.split();

    // Select between receiveing from the server and broadcasting messages received from the websocket
    select! {
        _ = handle_received_from_client(active_ws_mutex, read) => log::debug!("Message received from client"),
        _ = handle_received_from_server(rx, write) => log::debug!("Message retransmitted to client"),
    }

}

async fn handle_received_from_client(active_websockets: PeerMap, mut stream_read: SplitStream<WebSocketStream<TcpStream>>){
    // Waits for a message from the client and then broadcasts it to all the other
    // connected piers.

    match stream_read.next().await{
        Some(message_result) => {
            match message_result{
                Ok(message) => {
                    broadcast_message(message, active_websockets).await;
                },
                Err(_) => todo!(),
            }
        },
        None => {
            log::error!("Could not handle message received from client");
        },
    } 
}

async fn broadcast_message(message: Message, active_websockets: PeerMap){
    // Broadcasts a message to all clients connected in active_websockets

    for (addr, sender) in active_websockets.lock().await.iter(){
        let send_result = sender.send(message.clone());
        if send_result.is_err(){
            log::error!("Could not broadcast message to addr {addr}");
        }
    }
}


async fn handle_received_from_server(mut rx: UnboundedReceiver<Message>, mut write: SplitSink<WebSocketStream<TcpStream>, Message>){
    match rx.recv().await {
        Some(message) => {
            if write.send(message).await.is_err() {
                log::error!("Could not send message back to client");
            }
        }
        None => log::error!("Nothing came back from recv :(")
    };
}
