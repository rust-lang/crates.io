# Logging guidelines

This document captures the conventions for writing backend logs so they are
useful both to a human reading them and to queries in Datadog.

The backend logs through the [`tracing`](https://docs.rs/tracing) framework. In
production these logs are emitted as structured JSON and shipped to Datadog (see
[`ARCHITECTURE.md`](ARCHITECTURE.md#observability) for details). Write each
log line so it reads clearly on its own and exposes the values you will want to
search by.

## Message vs. fields

A log line has a free-text message and a set of structured key/value fields.
They serve different purposes and complement each other:

- The **message** describes what happened and must read clearly on its own.
  Interpolate the relevant identifiers into the text so two lines for different
  inputs do not look identical.
- **Fields** carry the values you will want to filter, group, or correlate on
  in Datadog. Fields make those values queryable. The message keeps them
  readable.

Because both views are useful, a key identifier usually appears in both: keep it
in the readable message and add it as a field.

```rust
// Bad: readable, but you cannot filter by crate in Datadog
info!("Generated OG image for crate `{name}`");

// Bad: filterable, but every line reads the same in the log view
info!(krate.name = %name, "Generated OG image");

// Good: readable in the log view and filterable by `krate.name`
info!(krate.name = %name, "Generated OG image for crate `{name}`");
```

Avoid vague messages with no context. `"operation failed"` tells the reader
nothing. Say which operation failed for what input.

## Field naming

- Use dotted names that group related fields by their domain entity:
  `krate.name`, `user.id`, `version.num`. We spell it `krate` because `crate` is
  a reserved word in Rust.
- Use the same name for the same thing everywhere, so a single query finds every
  occurrence.
- Put the unit in the name for measurements: `duration_ms`, `size_bytes`.
- Use `tracing`'s sigils to control how a value is recorded: `%value` for its
  `Display` representation, `?value` for its `Debug` representation.

## Log levels

The production default level is `INFO`, so `debug!` and `trace!` are only visible
when the level is raised locally or temporarily.

- `error!`: an operation failed and someone should look into it. Keep these
  rare and actionable. A flood of errors that nobody acts on trains everyone to
  ignore them. Log a failure once, at the boundary where it is handled, not at
  every layer it passes through.
- `warn!`: something unexpected happened but the system recovered or carried on
  in a degraded state (a fallback kicked in, a retry was needed, a config value
  is off). Worth watching for trends.
- `info!`: normal, high-level events describing what the system is doing, and
  the bulk of what we see in production. Keep it high-level rather than per-item
  detail.
- `debug!`: diagnostic detail for investigating a problem.
- `trace!`: very fine-grained detail, rarely needed.

## What never to log

Never put secrets or personal data into a message or a field. This includes:

- passwords and other credentials
- tokens, secrets, and session cookies
- personal data such as email addresses, IP addresses, or real names
- raw request or response bodies, which may contain any of the above
