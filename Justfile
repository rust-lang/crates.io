env := "development"
app := if env == "production" { "crates-io" } else if env == "staging"{ "staging-crates-io" } else { "" }

# List available commands
_default:
    just --list

# Error out if not an environment targeting Heroku
_assert_heroku:
    {{ if app == "" { error("Please specify env=staging or env=production") } else { "" } }}

# Squash the index (specify env=production or env=staging)
squash-index: _assert_heroku
    @echo Running an index squash against app: {{ app }}
    heroku run -a {{ app }} -- target/release/crates-admin enqueue-job squash_index
    @echo
    @echo There are several steps that must be done by hand:
    @echo 1. Fetch the index and push the snapshot branch to the "crates.io-index-archive" repo.
    @echo 2. Add a reminder to meeting agenda to drop the old branch. https://github.com/orgs/rust-lang/projects/25
    @echo 3. Post to the Zulip channel that the squash has been done. https://rust-lang.zulipchat.com/#narrow/stream/318791-t-crates-io/topic/index.20squashing

# This should be run each time we bump rustc, immediately before deploying
purge-heroku-build-cache: _assert_heroku
    @echo Puring the build cache for app: {{ app }}
    heroku builds:cache:purge -a {{ app }}
