# `crates_io_datadog`

This package implements functionality for submitting metrics to the
[Datadog metrics API](https://docs.datadoghq.com/api/latest/metrics/).

`DatadogClient::builder()` requires a `reqwest::Client` and API key, and
defaults to the `datadoghq.com` site. The client owns authentication, payload
serialization, request timeouts, and response validation.
