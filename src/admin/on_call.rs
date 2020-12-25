use anyhow::{anyhow, Result};
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
    pub fn send(self) -> Result<()> {
        #[cfg(not(test))]
        let base_url = "https://events.pagerduty.com";
        #[cfg(test)]
        let base_url = mockito::server_url();

        let api_token = dotenv::var("PAGERDUTY_API_TOKEN")?;
        let service_key = dotenv::var("PAGERDUTY_INTEGRATION_KEY")?;

        let url = format!("{}/generic/2010-04-15/create_event.json", base_url);
        let response = Client::new()
            .post(&url)
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
                Err(anyhow!("pagerduty error: {:?}", error))
            }
            Status::FORBIDDEN => Err(anyhow!("rate limited by pagerduty")),
            n => Err(anyhow!(
                "Got a non 200 response code from pagerduty: {} with {:?}",
                n,
                response
            )),
        }
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

#[cfg(test)]
mod tests {
    use super::Event;
    use mockito::{mock, Matcher};
    use std::env;

    #[test]
    fn test_send() {
        // set environment variables for this test
        env::set_var("PAGERDUTY_API_TOKEN", "secret123");
        env::set_var("PAGERDUTY_INTEGRATION_KEY", "crates-io-service-key");

        // setup the pagerduty API endpoint mock
        let response_body = json!({
            "description": "possible spam attack underway",
            "event_type": "trigger",
            "incident_key": "spam_attack",
            "service_key": "crates-io-service-key"
        });

        let mock = mock("POST", "/generic/2010-04-15/create_event.json")
            .match_header("Accept", "application/vnd.pagerduty+json;version=2")
            .match_header("Authorization", "Token token=secret123")
            .match_header("Content-Type", "application/json")
            .match_body(Matcher::Json(response_body))
            .with_status(200)
            .create();

        // create and send the event
        let event = Event::Trigger {
            incident_key: Some("spam_attack".into()),
            description: "possible spam attack underway".into(),
        };

        let result = event.send();

        // check that the mock endpoint was triggered
        mock.assert();
        assert_ok!(result);
    }

    #[test]
    fn test_send_with_400_error() {
        // set environment variables for this test
        env::set_var("PAGERDUTY_API_TOKEN", "secret123");
        env::set_var("PAGERDUTY_INTEGRATION_KEY", "crates-io-service-key");

        // setup the pagerduty API endpoint mock
        let request_body = json!({
            "message": "oops",
            "errors": ["something", "went", "wrong"],
        });

        let mock = mock("POST", "/generic/2010-04-15/create_event.json")
            .with_status(400)
            .with_body(request_body.to_string())
            .create();

        // create and send the event
        let event = Event::Trigger {
            incident_key: Some("spam_attack".into()),
            description: "possible spam attack underway".into(),
        };

        let result = event.send();

        // check that the mock endpoint was triggered
        mock.assert();
        assert_err!(result);
    }
}
