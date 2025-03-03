use std::time::Duration;
use simple_logger::SimpleLogger;
use futures_util::SinkExt;
use time::macros::format_description;
use tokio_tungstenite::connect_async;

#[tokio::main]
async fn main() {
    // Set default logging level
    if std::env::var("RUST_LOG").is_err() {
        std::env::set_var("RUST_LOG", "info")
    }

    println!("Logging level set to {:?}", std::env::var("RUST_LOG").unwrap());

    let url = match std::env::var("SERVE_IP"){
        Ok(value) => value,
        Err(_) => "ws://127.0.0.1:5050".to_string()
    };

    // Attempt to connect to server
    let (mut ws_stream, _) = connect_async(url)
        .await
        .expect("Failed to connect to server");

    // Init logger
    SimpleLogger::new()
        .with_level(log::LevelFilter::Trace)
        .env()
        .with_timestamp_format(format_description!("[year]-[month]-[day] [hour]:[minute]:[second]"))
        .init()
        .unwrap();

    // Send test message
    loop{ 
        if let Err(err) = ws_stream.send(tokio_tungstenite::tungstenite::Message::Text("hey".into())).await{
            log::error!("Could not send message to server: {err}");
        };
        tokio::time::sleep(Duration::from_secs(3)).await
    }

    // ws_stream.close(None).await;
}
