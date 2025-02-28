use simple_logger::SimpleLogger;
use tokio_tungstenite::connect_async;

#[tokio::main]
async fn main() {
    let url = std::env::args()
        .nth(1);
    if url.is_none(){
        log::error!("This program requires at least one argument");
        return;
    }

    // Attempt to connect to server
    let (ws_stream, _) = connect_async("127.0.0.1:5050")
        .await
        .expect("Failed to connect to server");



    // Init logger
    SimpleLogger::new()
        .with_level(log::LevelFilter::Trace)
        .env()
        .with_timestamp_format(format_description!("[year]-[month]-[day] [hour]:[minute]:[second]"))
        .init()
        .unwrap();

}
