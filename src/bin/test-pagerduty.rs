//! Send a test event to pagerduty
//!
//! Usage:
//!     cargo run --bin test-pagerduty event_type [description]
//!
//! Event type can be trigger, acknowledge, or resolve

#![deny(warnings, clippy::all, rust_2018_idioms)]

mod on_call;

use std::env::args;

fn main() {
    let args = args().collect::<Vec<_>>();

    let event_type = &*args[1];
    let description = args.get(2).cloned();

    let event = match event_type {
        "trigger" => on_call::Event::Trigger {
            incident_key: Some("test".into()),
            description: description.unwrap_or_else(|| "Test event".into()),
        },
        "acknowledge" => on_call::Event::Acknowledge {
            incident_key: "test".into(),
            description,
        },
        "resolve" => on_call::Event::Resolve {
            incident_key: "test".into(),
            description,
        },
        _ => panic!("Event type must be trigger, acknowledge, or resolve"),
    };
    event.send().unwrap()
}
