use cargo_registry::util::Error;

use reqwest::{blocking::Client, header, StatusCode as Status};

#[derive(serde::Serialize, Debug)]
#[serde(rename_all = "snake_case", tag = "event_type")]
pub enum Event {
    Trigger {
        incident_key: Option<String>,
        description: String,
    },
    #[allow(dead_code)] // Not all binaries create Acknowledge events
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
    pub fn send(self) -> Result<(), Error> {
        let api_token = dotenv::var("PAGERDUTY_API_TOKEN")?;
        let service_key = dotenv::var("PAGERDUTY_INTEGRATION_KEY")?;

        let response = Client::new()
            .post("https://events.pagerduty.com/generic/2010-04-15/create_event.json")
            .header(header::ACCEPT, "application/vnd.pagerduty+json;version=2")
            .header(header::AUTHORIZATION, format!("Token token={}", api_token))
            .json(&FullEvent {
                service_key,
                event: self,
            })
            .send()?;

        match response.status() {
            s if s.is_success() => Ok(()),
            Status::BAD_REQUEST => {
                let error = response.json::<InvalidEvent>()?;
                Err(format!("pagerduty error: {:?}", error))
            }
            Status::FORBIDDEN => Err("rate limited by pagerduty".to_string()),
            n => Err(format!(
                "Got a non 200 response code from pagerduty: {} with {:?}",
                n, response
            )),
        }
        .map_err(Into::into)
    }
}

#[derive(serde::Serialize, Debug)]
struct FullEvent {
    service_key: String,
    #[serde(flatten)]
    event: Event,
}

#[derive(serde::Deserialize, Debug)]
struct InvalidEvent {
    message: String,
    errors: Vec<String>,
}
