use std::{
    env,
    io::Read,
    net::{TcpListener, TcpStream},
    str::FromStr,
};

use serde::Deserialize;
use tracing::Level;

#[derive(Debug, Deserialize)]
struct CameraEvent<'a> {
    #[serde(rename = "Type")]
    event_type: &'a str,
    #[serde(rename = "Status")]
    status: i32,
    #[serde(rename = "Time")]
    time: &'a str,
    #[serde(rename = "IP")]
    ip: &'a str,
    #[serde(rename = "DeviceName")]
    device_name: &'a str,
}

fn main() -> color_eyre::Result<()> {
    color_eyre::install()?;

    let listen_addr = env::var("ALAAARM_LISTEN");
    let listen_addr = listen_addr.as_deref().unwrap_or("0.0.0.0:6060");
    let log_level = env::var("ALAAARM_LOG")
        .ok()
        .and_then(|x| Level::from_str(&x).ok())
        .unwrap_or(Level::INFO);

    tracing_subscriber::fmt::fmt()
        .with_max_level(log_level)
        .init();

    let listen = TcpListener::bind(listen_addr)?;
    tracing::info!("Listening on {listen_addr}");
    for conn in listen.incoming() {
        if let Err(e) = handle_session(conn?) {
            tracing::error!("error in connection handler: {e:?}");
        }
    }
    Ok(())
}

fn handle_session(mut stream: TcpStream) -> color_eyre::Result<()> {
    let mut json = String::new();
    stream.read_to_string(&mut json)?;
    let json = json.strip_suffix("\0").unwrap_or(&json);
    tracing::info!("Read JSON: {json:?}");
    let event: CameraEvent = serde_json::from_str(json)?;
    tracing::info!("Parsed as: {event:?}");
    Ok(())
}
