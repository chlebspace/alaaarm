use serde::{Deserialize, Serialize};
use ureq::{Agent, config::Config};

pub struct Frigate {
    http: Agent,
    base_url: String,
}

#[derive(Debug, Deserialize)]
struct CreateEventResponse {
    success: bool,
    event_id: String,
    message: String,
}

#[derive(Debug, Deserialize)]
struct EndEventResponse {
    success: bool,
    message: String,
}

#[derive(Serialize)]
struct LoginRequest<'a> {
    user: &'a str,
    password: &'a str,
}

impl Frigate {
    pub fn new(url: String) -> Self {
        let http_config = Config::builder()
            .user_agent(concat!("alaaarm/", env!("CARGO_PKG_VERSION")))
            .build();
        Self {
            http: Agent::new_with_config(http_config),
            base_url: url,
        }
    }

    pub fn login(&self, user: &str, password: &str) -> color_eyre::Result<()> {
        tracing::info!("logging in to Frigate");
        let body = LoginRequest { user, password };
        self.http
            .post(format!("{}/login", self.base_url))
            .send_json(body)?;
        tracing::info!("login successful");
        Ok(())
    }

    pub fn create_event(&self, camera_name: &str, label: &str) -> color_eyre::Result<String> {
        let resp: CreateEventResponse = self
            .http
            .post(format!(
                "{}/events/{camera_name}/{label}/create",
                self.base_url
            ))
            .send_empty()?
            .into_body()
            .read_json()?;
        tracing::debug!("{resp:?}");
        Ok(resp.event_id)
    }

    pub fn end_event(&self, event_id: &str) -> color_eyre::Result<()> {
        let resp: EndEventResponse = self
            .http
            .put(format!("{}/events/{event_id}/end", self.base_url))
            .send_empty()?
            .into_body()
            .read_json()?;
        tracing::debug!("{resp:?}");
        Ok(())
    }
}
