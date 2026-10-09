#![doc = include_str!("../README.md")]

use async_trait::async_trait;
use bon::Builder;
use reqwest::StatusCode;
use secrecy::{ExposeSecret, SecretString};
use serde::Deserialize;
use url::Url;

#[derive(Debug, thiserror::Error)]
pub enum ZulipError {
    #[error("Failed to send Zulip API request")]
    Request(#[from] reqwest::Error),

    #[error("Zulip bot email `{0}` does not contain a domain")]
    InvalidBotEmail(String),

    #[error("Zulip API responded with {status}: {message}")]
    Api { status: StatusCode, message: String },
}

#[cfg_attr(feature = "mock", mockall::automock)]
#[async_trait]
pub trait ZulipClient: Send + Sync {
    /// Sends a message to the given topic of a Zulip channel.
    async fn send_channel_message(
        &self,
        channel: &str,
        topic: &str,
        content: &str,
    ) -> Result<(), ZulipError>;
}

/// [`ZulipClient`] implementation that authenticates as a Zulip bot.
#[derive(Builder)]
pub struct RealZulipClient {
    #[builder(default = default_http_client())]
    client: reqwest::Client,

    /// Base URL of the Zulip instance.
    ///
    /// Defaults to the domain of the bot email address, which Zulip Cloud
    /// sets to the host of the organization.
    base_url: Option<Url>,

    #[builder(into)]
    bot_email: String,

    api_key: SecretString,
}

fn bot_email_base_url(bot_email: &str) -> Option<Url> {
    let (_, domain) = bot_email.rsplit_once('@')?;
    if domain.is_empty() {
        return None;
    }

    format!("https://{domain}").parse().ok()
}

fn default_http_client() -> reqwest::Client {
    reqwest::Client::builder()
        .user_agent(crates_io_version::user_agent())
        .build()
        .unwrap()
}

#[async_trait]
impl ZulipClient for RealZulipClient {
    async fn send_channel_message(
        &self,
        channel: &str,
        topic: &str,
        content: &str,
    ) -> Result<(), ZulipError> {
        let base_url = match &self.base_url {
            Some(base_url) => base_url.clone(),
            None => bot_email_base_url(&self.bot_email)
                .ok_or_else(|| ZulipError::InvalidBotEmail(self.bot_email.clone()))?,
        };

        let url = base_url.join("/api/v1/messages").unwrap();
        let form = [
            ("type", "stream"),
            ("to", channel),
            ("topic", topic),
            ("content", content),
        ];

        let response = self
            .client
            .post(url)
            .basic_auth(&self.bot_email, Some(self.api_key.expose_secret()))
            .form(&form)
            .send()
            .await?;

        let status = response.status();
        if status.is_success() {
            return Ok(());
        }

        #[derive(Deserialize)]
        struct ErrorResponse {
            msg: String,
        }

        let body = response.text().await?;
        let message = match serde_json::from_str::<ErrorResponse>(&body) {
            Ok(error) => error.msg,
            Err(_) => body,
        };

        Err(ZulipError::Api { status, message })
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use claims::{assert_matches, assert_none, assert_ok};
    use mockito::Matcher;

    fn client(server: &mockito::Server) -> RealZulipClient {
        RealZulipClient::builder()
            .base_url(server.url().parse().unwrap())
            .bot_email("bot@example.com")
            .api_key("secret".into())
            .build()
    }

    fn mock_send(server: &mut mockito::Server) -> mockito::Mock {
        server
            .mock("POST", "/api/v1/messages")
            // `bot@example.com:secret`
            .match_header("Authorization", "Basic Ym90QGV4YW1wbGUuY29tOnNlY3JldA==")
            .match_body(Matcher::AllOf(vec![
                Matcher::UrlEncoded("type".into(), "stream".into()),
                Matcher::UrlEncoded("to".into(), "t-crates-io".into()),
                Matcher::UrlEncoded("topic".into(), "index squashing".into()),
                Matcher::UrlEncoded("content".into(), "Hello **world**!".into()),
            ]))
    }

    async fn send(client: &RealZulipClient) -> Result<(), ZulipError> {
        client
            .send_channel_message("t-crates-io", "index squashing", "Hello **world**!")
            .await
    }

    #[tokio::test]
    async fn test_success() {
        let mut server = mockito::Server::new_async().await;
        let mock = mock_send(&mut server)
            .with_body(r#"{"result":"success","msg":"","id":42}"#)
            .create_async()
            .await;

        assert_ok!(send(&client(&server)).await);
        mock.assert_async().await;
    }

    #[tokio::test]
    async fn test_api_error() {
        let mut server = mockito::Server::new_async().await;
        mock_send(&mut server)
            .with_status(400)
            .with_body(r#"{"result":"error","msg":"Channel 't-crates-io' does not exist","code":"STREAM_DOES_NOT_EXIST"}"#)
            .create_async()
            .await;

        assert_matches!(
            send(&client(&server)).await,
            Err(ZulipError::Api { status: StatusCode::BAD_REQUEST, message })
                if message == "Channel 't-crates-io' does not exist"
        );
    }

    #[tokio::test]
    async fn test_unexpected_error_body() {
        let mut server = mockito::Server::new_async().await;
        mock_send(&mut server)
            .with_status(502)
            .with_body("Bad Gateway")
            .create_async()
            .await;

        assert_matches!(
            send(&client(&server)).await,
            Err(ZulipError::Api { status: StatusCode::BAD_GATEWAY, message })
                if message == "Bad Gateway"
        );
    }

    #[test]
    fn test_bot_email_base_url() {
        let base_url = bot_email_base_url("crates-io-bot@rust-lang.zulipchat.com").unwrap();
        assert_eq!(base_url.as_str(), "https://rust-lang.zulipchat.com/");

        assert_none!(bot_email_base_url("crates-io-bot"));
        assert_none!(bot_email_base_url("crates-io-bot@"));
    }

    #[tokio::test]
    async fn test_invalid_bot_email() {
        let client = RealZulipClient::builder()
            .bot_email("crates-io-bot")
            .api_key("secret".into())
            .build();

        assert_matches!(
            send(&client).await,
            Err(ZulipError::InvalidBotEmail(email)) if email == "crates-io-bot"
        );
    }
}
