# crates_io_api_types

This crate contains some of the shared API response and request types used by the crates.io API.

These types are serialized to/from JSON and represent the public API surface that clients interact with. They are distinct from the database models in `crates_io_database`, which represent the internal database schema.

The crate includes types for publishing crates, trusted publishing configuration, release tracking metadata, and various encodable domain objects (crates, versions, users, teams, categories, keywords).

## Design principles

- **No business logic**: Types are primarily data structures with minimal behavior beyond serialization
- **OpenAPI schema**: Types include `utoipa::ToSchema` derives for automatic OpenAPI documentation generation
