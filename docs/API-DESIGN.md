# JSON API design guidelines

This document captures the conventions used by the crates.io JSON API so new
endpoints fit alongside existing ones. It describes the patterns we follow
today, not where we want to be. Inconsistencies are noted in the "Known
quirks" section at the end so new code knows which side to take.

The API is consumed by three groups of clients:

- the `cargo` CLI and other third-party clients (stable, conservative)
- the crates.io SvelteKit frontend (private endpoints allowed)
- third-party tooling reading the public OpenAPI spec

Treat the first group as a hard backwards-compatibility constraint. The
second group's endpoints may be marked internal (see [OpenAPI](#openapi)).

## URL structure

- All public endpoints live under `/api/v1/`. There is no v2.
- Some internal-only endpoints live under `/api/private/` and are registered
  outside the OpenAPI router.
- Collections use plural nouns: `/crates`, `/users`, `/keywords`,
  `/api_tokens`.
- A single item reuses the collection path with an identifier:
  `/crates/{name}`, `/users/{id}`.
- Sub-resources nest under their parent: `/crates/{name}/owners`,
  `/crates/{name}/{version}/downloads`.
- The identifier in the path is the stable human key (crate name, slug,
  login) where one exists, otherwise the numeric id.

Wire routes in `src/router.rs` using `OpenApiRouter` and group methods
sharing a path with `routes!()`.

## HTTP methods

| Method | Use |
| --- | --- |
| `GET` | Reads (list and show). Never has side effects. |
| `POST` | Create operations driven by the frontend. |
| `PUT` | Create-or-set operations driven by `cargo` (publish, add owner, follow). |
| `PATCH` | Partial updates. |
| `DELETE` | Removal, plus historical cargo actions (yank, unfollow). |

When a new endpoint has no `cargo` client constraint, prefer `POST` for
creation and `PATCH` for partial updates.

## Path and query parameters

Path parameters are bound through a per-resource extractor struct that
derives `Deserialize`, `FromRequestParts`, and `utoipa::IntoParams`. Reuse
`CratePath` (`src/controllers/krate.rs`) and `CrateVersionPath`
(`src/controllers/version.rs`) where they apply; the version extractor
already validates semver in `deserialize_with`.

Query parameters use a per-handler `*QueryParams` struct with
`#[from_request(via(Query))]`. List the struct in the `params(...)` clause
of `#[utoipa::path]` alongside `PaginationQueryParams` when applicable.

Repeated array params (e.g. `ids[]=a&ids[]=b`) require
`axum_extra::extract::Query` and `#[serde(rename = "ids[]")]` on the field.

## Request bodies

JSON bodies use the `Json<T>` extractor with a per-handler request struct.
Wrap the body in a single top-level key naming the resource:

```json
{ "version": { "yanked": true, "yank_message": "..." } }
```

`snake_case` throughout. Validate inside the handler and return
`bad_request("...")` for client errors; there is no shared validator
crate.

The only non-JSON body in the API is the publish tarball, which uses the
raw `axum::body::Body` extractor.

## Response bodies

All successful responses are JSON objects with a stable top-level shape.

**Single resource** — one key named after the resource:

```json
{ "crate": { "id": "serde", "name": "serde", ... } }
```

**Collection** — plural key plus a sibling `meta` object:

```json
{
  "crates": [ { ... }, { ... } ],
  "meta": { "total": 1234, "next_page": "?page=3", "prev_page": null }
}
```

**Action without payload** — the shared `OkResponse` from
`src/controllers/helpers.rs`:

```json
{ "ok": true }
```

Conventions for the payload itself:

- `snake_case` keys.
- Timestamps as RFC 3339 UTC with a `Z` suffix: `"2017-01-06T14:23:11Z"`.
  Serialize as `chrono::DateTime<Utc>`.
- Optional values serialize as JSON `null` by default. Use
  `#[serde(skip_serializing_if = "Option::is_none")]` only when the field
  is genuinely not part of the shape (e.g. expansion-only fields).
- For new endpoints, `id` is the database id. Expose the human-readable
  identifier (slug, name, login) under a separate field. Existing
  resources like `EncodableCrate` and `EncodableKeyword` use the human key
  as `id` for historical reasons; do not copy that pattern.

Reusable response shapes belong in `crates/crates_io_api_types`, named
`Encodable*` and renamed via `#[schema(as = Foo)]` so the OpenAPI
component name drops the prefix. One-off response structs live next to the
handler.

## Including related resources

Endpoints may accept an `?include=` query parameter that lets clients pull
in related resources in the same response, as `find_crate`
(`src/controllers/krate/metadata.rs`) does today. Conventions:

- Default to not including anything. Clients opt in field by field.
- Accept a comma-separated list of known field names. Reject unknown
  names with `bad_request`.
- Model include-gated fields as `Option<T>` with
  `#[serde(skip_serializing_if = "Option::is_none")]` so they are omitted
  from the response unless the client asked for them. `find_crate`
  predates this guidance and emits `null` instead; do not copy that.

This adds real complexity to the handler and the response type, so reach
for it only when a client genuinely needs to fan out one request into
several. A second endpoint is often the simpler answer.

## Pagination

Pagination is centralized in `src/controllers/helpers/pagination.rs`.
Accept `PaginationQueryParams` and pick a scheme:

- **Seek pagination** (preferred for new endpoints). Disable offset
  pagination with `enable_pages(false)`.
- **Offset pagination** for legacy endpoints. `page` past
  `MAX_PAGE_BEFORE_SUSPECTED_BOT` (10) must fall back to seek.

Defaults: `per_page = 10`, max `100`. Return `meta.next_page` (and
`meta.prev_page` for offset) as a full query string starting with `?`, so
clients can append it to the base URL verbatim. `meta.total` is `i64` and
always present.

## Error responses

Every error response uses the same envelope, produced by `json_error` in
`src/util/errors/json.rs`:

```json
{ "errors": [{ "detail": "..." }] }
```

Handlers return `AppResult<T>`. Build errors via the helpers in
`src/util/errors.rs`:

| Helper | Status |
| --- | --- |
| `bad_request` | 400 |
| `forbidden`, `account_locked` | 403 |
| `not_found`, `crate_not_found`, `version_not_found` | 404 |
| `server_error` | 500 |
| `service_unavailable` | 503 |
| `custom(status, detail)` | anything else (e.g. 409, 429) |

`429` responses include a `Retry-After` header and a link to the rate
limit docs in the detail message. There are no `X-RateLimit-*` headers.

The `cargo_compat` middleware (`src/middleware/cargo_compat.rs`) rewrites
plain-text errors to JSON and downgrades some statuses to 200 for old
`cargo` versions. Do not depend on it from new code; return the correct
status and shape directly.

## Authentication

Three security schemes are registered in `SecurityAddon`
(`src/openapi.rs`):

- `cookie` — session cookie used by the frontend.
- `api_token` — `Authorization` header, used by `cargo` and third-party
  clients.
- `trustpub_token` — Bearer token, accepted only on publish.

Enforce auth with `AuthCheck` (`src/auth.rs`):

- `AuthCheck::default()` accepts either cookie or API token.
- `AuthCheck::only_cookie()` restricts to the web frontend.
- `AuthCheck::default().with_endpoint_scope(...).for_crate(name)` checks
  that scoped API tokens carry the matching endpoint scope and crate
  scope. Legacy (unscoped) tokens still pass.

Declare auth on the handler with the `security(...)` clause of
`#[utoipa::path]`. Patterns:

```text
security(("api_token" = []), ("cookie" = []))                 // required, either scheme
security(("cookie" = []))                                     // web only
security((), ("api_token" = []), ("cookie" = []))             // optional auth
```

## OpenAPI

Every public handler carries a `#[utoipa::path]` attribute. Always set
`method`, `path`, `operation_id` (via the handler name), `tag`, and
`responses`. Add `params`, `request_body`, and `security` as the endpoint
requires.

- The operation id comes from the handler function name. Use
  `<verb>_<resource>[_<modifier>]`: `list_crates`, `find_crate`,
  `create_token`, `update_version`, `delete_crate`, `yank_version`.
- `responses` documents only the success case as
  `(status = 200, body = inline(ResponseType))`. Errors follow the global
  envelope and are currently not enumerated per endpoint.
- The `tag` groups endpoints by domain, not URL prefix. Existing tags:
  `crates`, `versions`, `owners`, `users`, `teams`, `keywords`,
  `categories`, `api_tokens`, `session`, `trusted_publishing`, `publish`,
  `other`. Reuse one of these unless a new domain genuinely emerges.
- Endpoints used only by the frontend should be marked internal with
  `extensions(("x-internal" = json!(true)))`. They are filtered out of the
  default OpenAPI document and exposed via `?internal=...`.
- Deprecate fields with `#[schema(deprecated)]`. We do not emit
  `Deprecation` or `Sunset` HTTP headers.

The full OpenAPI document is locked by snapshot tests; run
`cargo test --package crates_io --lib openapi` and accept changes with
`cargo insta accept`.

## Testing

Capture response shapes with `insta` snapshots. Place tests next to the
controller area (`src/tests/<area>/`) and let snapshots cover the JSON
body. Use `[datetime]` and similar redactions for non-deterministic
fields.

## Known quirks

These exist for historical reasons. Do not propagate them to new endpoints
unless the use case genuinely matches.

- **Yank uses `DELETE` and `PUT`**: `DELETE /crates/{n}/{v}/yank` and
  `PUT /crates/{n}/{v}/unyank` come from cargo. New action endpoints
  should use `POST` on a verb sub-resource or `PATCH` on the resource.
- **`PUT /users/{user}` is a partial update.** Prefer `PATCH` for new
  partial updates.
- **Singular sub-resources** like `/crates/{n}/owner_team`,
  `/crates/{n}/owner_user`, `/crates/{n}/follow` predate the plural
  convention.
- **Two list meta shapes**: most lists return `{total, next_page,
  prev_page}`; a few older ones return `{more: bool}`. Use the former.
- **Embedded `"links"` objects** on `EncodableCrate`, `EncodableVersion`,
  etc. point at sub-resource paths. Nothing consumes them today; do not
  add a `links` object to new response types.
