use anyhow::Result;
use std::str::FromStr;

use crates_io_pagerduty as pagerduty;

#[derive(Debug, Copy, Clone, clap::ValueEnum)]
pub enum EventType {
    Trigger,
    Acknowledge,
    Resolve,
}

impl FromStr for EventType {
    type Err = &'static str;

    fn from_str(value: &str) -> Result<Self, Self::Err> {
        match value {
            "trigger" => Ok(EventType::Trigger),
            "acknowledge" => Ok(EventType::Acknowledge),
            "resolve" => Ok(EventType::Resolve),
            _ => Err("Event type must be trigger, acknowledge, or resolve"),
        }
    }
}

#[derive(clap::Parser, Debug)]
#[command(name = "test-pagerduty", about = "Send a test event to pagerduty")]
pub struct Opts {
    #[arg(value_enum)]
    event_type: EventType,
    description: Option<String>,
}

pub async fn run(opts: Opts) -> Result<()> {
    let event = match opts.event_type {
        EventType::Trigger => pagerduty::Event::Trigger {
            incident_key: Some("test".into()),
            description: opts.description.unwrap_or_else(|| "Test event".into()),
        },
        EventType::Acknowledge => pagerduty::Event::Acknowledge {
            incident_key: "test".into(),
            description: opts.description,
        },
        EventType::Resolve => pagerduty::Event::Resolve {
            incident_key: "test".into(),
            description: opts.description,
        },
    };
    event.send().await
}
