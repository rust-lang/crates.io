# `crates_io_github_app`

Mints installation access tokens for a GitHub App, used by the
background worker to:

- authenticate HTTPS pushes to the archive index repository
- authenticate requests to the users API to get higher rate limits

The `GitHubApp` trait abstracts the HTTP interaction for testing. The
`GitHubAppClient` struct is the actual implementation that signs a JWT
with the app's private key, resolves the installation id once, and caches
the minted installation access token until shortly before its expiry.
