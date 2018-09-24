use curl::easy::*;
use serde_json;
use std::env;
use std::io::prelude::*;

use util::*;

#[derive(Serialize, Debug)]
#[serde(rename_all = "snake_case", tag = "event_type")]
pub enum Event {
    Trigger {
        incident_key: Option<String>,
        description: String,
    },
    Acknowledge {
        incident_key: String,
        description: Option<String>,
    },
    Resolve {
        incident_key: String,
        description: Option<String>,
    },
}

impl Event {
    /// Sends the event to pagerduty.
    ///
    /// If the variant is `Trigger`, this will page whoever is on call
    /// (potentially waking them up at 3 AM).
    pub fn send(self) -> CargoResult<()> {
        let api_token = env::var("PAGERDUTY_API_TOKEN")?;
        let service_key = env::var("PAGERDUTY_INTEGRATION_KEY")?;

        let mut headers = List::new();
        headers.append("Accept: application/vnd.pagerduty+json;version=2")?;
        headers.append(&format!("Authorization: Token token={}", api_token))?;
        headers.append("Content-Type: application/json")?;

        let mut handle = Easy::new();
        handle.url("https://events.pagerduty.com/generic/2010-04-15/create_event.json")?;
        handle.post(true)?;
        handle.http_headers(headers)?;

        let full_event = FullEvent { service_key, event: self };
        let json_body = serde_json::to_string(&full_event)?;
        let mut bytes_to_write = json_body.as_bytes();
        let mut data = Vec::new();

        {
            let mut handle = handle.transfer();
            handle.read_function(|bytes| {
                bytes_to_write.read(bytes).map_err(|_| ReadError::Abort)
            })?;
            handle.write_function(|buf| {
                data.extend_from_slice(buf);
                Ok(buf.len())
            })?;
            handle.perform()?;
        }

        match handle.response_code()? {
            200 => Ok(()),
            400 => {
                let error = serde_json::from_slice::<InvalidEvent>(&data)?;
                Err(internal(&format_args!("pagerduty error: {:?}", error)))
            },
            403 => Err(internal("rate limited by pagerduty")),
            n => {
                let resp = String::from_utf8_lossy(&data);
                Err(internal(&format_args!(
                    "Got a non 200 response code from pagerduty: {} with {}", n, resp)))
            }
        }
    }
}

#[derive(Serialize, Debug)]
struct FullEvent {
    service_key: String,
    #[serde(flatten)]
    event: Event,
}

#[derive(Deserialize, Debug)]
struct InvalidEvent {
    message: String,
    errors: Vec<String>,
}
