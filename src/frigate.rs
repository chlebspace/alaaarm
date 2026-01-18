use std::borrow::Cow;

use anyhow::bail;
use percent_encoding::{NON_ALPHANUMERIC, utf8_percent_encode};
use serde::{Deserialize, Serialize, de::DeserializeOwned};
use serde_json::Map;
use ureq::{Agent, Body, config::Config, http::Response};

pub struct Frigate {
    http: Agent,
    base_url: String,
    user: String,
    password: String,
}

#[derive(Debug, Deserialize)]
#[allow(unused)] // fields used by Debug
struct CreateEventResponse {
    success: bool,
    event_id: String,
    message: String,
}

#[derive(Debug, Deserialize)]
#[allow(unused)] // fields used by Debug
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
    pub fn new(base_url: String, user: String, password: String) -> Self {
        let http_config = Config::builder()
            .user_agent(concat!("alaaarm/", env!("CARGO_PKG_VERSION")))
            .build();
        Self {
            http: Agent::new_with_config(http_config),
            base_url,
            user,
            password,
        }
    }

    pub fn login(&self) -> anyhow::Result<()> {
        tracing::info!("logging in to Frigate");

        self.http
            .post(format!("{}login", self.base_url))
            .send_json(LoginRequest {
                user: &self.user,
                password: &self.password,
            })?;

        tracing::info!("login successful");
        Ok(())
    }

    pub fn create_event(&self, camera_name: &str, label: &str) -> anyhow::Result<String> {
        let camera_name: Cow<str> = utf8_percent_encode(camera_name, NON_ALPHANUMERIC).into();
        let label: Cow<str> = utf8_percent_encode(label, NON_ALPHANUMERIC).into();

        let resp: CreateEventResponse = self.request_with_login(|h| {
            h.post(format!(
                "{}events/{camera_name}/{label}/create",
                self.base_url
            ))
            .send_empty()
        })?;

        tracing::debug!("{resp:?}");
        Ok(resp.event_id)
    }

    pub fn end_event(&self, event_id: &str) -> anyhow::Result<()> {
        let event_id: Cow<str> = utf8_percent_encode(event_id, NON_ALPHANUMERIC).into();

        let resp: EndEventResponse = self.request_with_login(|h| {
            h.put(format!("{}events/{event_id}/end", self.base_url))
                .send_json(Map::new())
        })?;

        tracing::debug!("{resp:?}");
        Ok(())
    }

    // send request, but log in and try again in case of 401
    fn request_with_login<B, F>(&self, make_req: F) -> anyhow::Result<B>
    where
        B: DeserializeOwned,
        F: Fn(&Agent) -> Result<Response<Body>, ureq::Error>,
    {
        let resp = match make_req(&self.http) {
            Ok(o) => o,
            Err(ureq::Error::StatusCode(401)) => {
                tracing::info!("received 401 unauthorized, logging in again and retrying");
                self.login()?;
                make_req(&self.http)?
            }
            Err(e) => bail!(e),
        }
        .into_body()
        .read_json()?;

        Ok(resp)
    }
}
