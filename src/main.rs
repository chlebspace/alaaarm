use std::{
    collections::HashMap,
    io::Read,
    net::{TcpListener, TcpStream},
    sync::mpsc,
    thread,
};

use color_eyre::eyre::bail;
use dotenv::dotenv;
use serde::Deserialize;

use crate::{config::Config, frigate::Frigate};

mod config;
mod frigate;

#[derive(Debug, Deserialize)]
pub struct CameraEvent<'a> {
    #[serde(rename = "Type")]
    pub kind: &'a str,
    #[serde(rename = "Status")]
    pub status: i32,
    #[serde(rename = "Time")]
    pub time: &'a str,
    #[serde(rename = "IP")]
    pub ip: &'a str,
    #[serde(rename = "DeviceName")]
    pub device_name: &'a str,
}

struct AppState {
    frigate: Frigate,
    // device name -> current event ID
    pending: HashMap<String, String>,
}

fn main() -> color_eyre::Result<()> {
    color_eyre::install()?;

    if let Err(e) = dotenv()
        && !e.not_found()
    {
        bail!(e)
    }

    let config = Config::load_from_env()?;

    tracing_subscriber::fmt::fmt()
        .with_max_level(config.log_level)
        .init();

    let listen = TcpListener::bind(&config.listen_addr)?;

    let frigate = Frigate::new(config.frigate_url);
    frigate.login(&config.frigate_user, &config.frigate_password)?;

    let state = AppState {
        frigate,
        pending: HashMap::new(),
    };

    let (task_tx, task_rx) = mpsc::sync_channel(10);
    thread::spawn(move || handler_loop(task_rx, state));

    tracing::info!("Listening on {}", config.listen_addr);
    for stream in listen.incoming() {
        match stream {
            Ok(stream) => task_tx.send(stream)?,
            Err(err) => tracing::error!("connection failed with error: {err:?}"),
        }
    }
    Ok(())
}

fn handler_loop(task_rx: mpsc::Receiver<TcpStream>, mut state: AppState) {
    for stream in task_rx {
        if let Err(e) = handle_session(stream, &mut state) {
            tracing::error!("error in connection handler: {e:?}");
        }
    }
}

fn handle_session(mut stream: TcpStream, state: &mut AppState) -> color_eyre::Result<()> {
    let mut json = String::with_capacity(200);
    stream.read_to_string(&mut json)?;
    let Some(json) = json.strip_suffix("\0") else {
        bail!("expected null-terminated JSON, got: {json:?}")
    };
    tracing::debug!("read JSON: {json:?}");
    let event: CameraEvent = serde_json::from_str(json)?;
    tracing::info!("{event:?}");
    match event.status {
        // object is being tracked
        1 => {
            if state.pending.contains_key(event.device_name) {
                tracing::warn!(
                    "device {} tried to create a new event without ending previous",
                    event.device_name
                );
                return Ok(());
            }
            let event_id = state.frigate.create_event(event.device_name, event.kind)?;
            tracing::info!("event created: {}", event_id);
            state.pending.insert(event.device_name.into(), event_id);
        }
        // object is not being tracked anymore
        0 => {
            let Some(event_id) = state.pending.remove(event.device_name) else {
                tracing::warn!(
                    "device {} tried to end a nonexistent event",
                    event.device_name
                );
                return Ok(());
            };
            state.frigate.end_event(&event_id)?;
            tracing::info!("event {event_id} ended");
        }
        invalid => bail!("expected status 0 or 1, got {invalid}"),
    }

    Ok(())
}
