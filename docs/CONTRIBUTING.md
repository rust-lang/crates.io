# Contributing to crates.io

- [Attending the weekly team meetings](#attending-the-weekly-team-meetings)
- [Finding an issue to work on](#finding-an-issue-to-work-on)
- [Submitting a Pull Request](#submitting-a-pull-request)
- [Reviewing Pull Requests](#reviewing-pull-requests)
- [Checking Release State](#checking-release-state)
- [Setting up a development environment](#setting-up-a-development-environment)
  - [Working on the Frontend](#working-on-the-frontend)
  - [Working on the Backend](#working-on-the-backend)
  - [Running crates.io with Docker](#running-cratesio-with-docker)

## Attending the weekly team meetings

Each Friday at 11:00am US east coast time the crates.io team gets together
on Zoom or [Zulip] (`#t-crates-io`) for our weekly team meeting, and we invite
everyone who wants to contribute to crates.io to participate.

[Zulip]: https://rust-lang.zulipchat.com/#narrow/stream/318791-t-crates-io/

## Finding an issue to work on

We try to keep a variety of issues tagged with
[E-has-mentor](https://github.com/rust-lang/crates.io/issues?q=is%3Aopen+is%3Aissue+label%3AE-has-mentor).
These issues should contain, somewhere within the body or the comments, clear
instructions on what needs to be done and where the changes will need to be
made. If any E-has-mentor issues do not contain this information or the information
isn't clear, we consider this a bug, please comment and ask for clarification!
Please don't hesitate to ask any questions on these issues, we're paying special
attention to them and someone will get back to you with help as soon as
possible.

We'd also love contributions for issues not tagged `E-has-mentor`, they just might
not be as well specified. You may want to browse through the labels that start
with A-, which stands for "area", to find issues that match with your interests.

If you'd like to work on something that isn't in a current issue, especially if
it would be a big change, please open a new issue for discussion!

## Submitting a Pull Request

As an initiative to improve the documentation of the crates.io codebase, we would
like to see all new types and functions, public and private, to have documentation
comments on them. If you change an existing type or function, and it doesn't have
a documentation comment on it, it'd be great if you could add one to it too.

When you submit a pull request, it will be automatically tested on GitHub Actions. In
addition to running both the frontend and the backend tests described below,
GitHub Actions runs [ESLint], [clippy], and [rustfmt] on each PR.

If you don't want to run these tools locally, please watch the GitHub Actions results
and submit additional commits to your pull request to fix any issues they find!

If you do want to run these tools locally in order to fix issues before
submitting, that would be great too! Please consult each tool's installation
instructions and the [.github/workflows/ci.yml] file in this repository for the latest
installation and running instructions. The logs for recent builds in GitHub Actions
may also be helpful to see which versions of these tools we're currently using.

[ESLint]: https://eslint.org/
[clippy]: https://github.com/rust-lang-nursery/rust-clippy
[rustfmt]: https://github.com/rust-lang-nursery/rustfmt
[.github/workflows/ci.yml]: /.github/workflows/ci.yml

We will try to review your pull requests as soon as possible!

## Reviewing Pull Requests

Another way to help out and to get to know the codebase is to review other people's
pull requests! Take a look at [`docs/PR-REVIEW.md`](https://github.com/rust-lang/crates.io/blob/master/docs/PR-REVIEW.md)
for guidelines on how to do that.

## Checking Release State

If you are curious on the version that is published for crates.io, you can
visit [whatsdeployed.io](https://whatsdeployed.io/s/FEM/rust-lang/crates.io).
There you will find the commits involved in the current release.

If you are only interested on the commit hash, you can also hit the site
metadata endpoint available on `/api/v1/site_metadata`.

## Setting up a development environment

First, you'll need git to clone the repo. [GitHub has help pages about setting
up git](https://help.github.com/articles/set-up-git/), and once you've done
that, you should be able to clone the repo and change into the repo's directory
from your terminal:

```console
git clone https://github.com/rust-lang/crates.io.git
cd crates.io/
```

## Working on the Frontend

If the changes you'd like to make only involve:

- HTML
- JavaScript
- CSS

and don't need:

- any changes to the JSON returned by the requests to the backend
- a user to be logged in

then you only need to set up the frontend. Once you've set up a local frontend,
you can run your frontend against the production API.

If you need to set up the backend, you'll probably want to set up the frontend
as well.

### Frontend requirements

In order to run the frontend on Windows and macOS, you will need to have installed:

- [node](https://nodejs.org/en/) (see `package.json` engines field for the exact version required)
- [pnpm](https://pnpm.io) >= 8.5.1

Follow the links for each of these tools for their recommended installation
instructions. If you already have these tools, or you have a different
preferred method of installing packages like these, that should work fine.

If you are on Linux, use [nvm](https://github.com/creationix/nvm/blob/master/README.md)
to ensure that the use of `npm` does not require the use of `sudo`.

The front end should run fine after these steps. Please file an issue if you run
into any trouble.

### Building and serving the frontend

To install the npm packages that crates.io uses, run:

```console
pnpm install
```

You'll need to run these commands any time the libraries or versions of these
libraries that crates.io uses change. Usually you'll know they've changed
because you'll run the next step and it will fail saying it can't find some
libraries.

To build and serve the frontend assets, use the command `pnpm start`. There
are variations on this command that change which backend your frontend tries to
talk to:

| Command                                   | Backend                                   | Use case                                                |
| ----------------------------------------- | ----------------------------------------- | ------------------------------------------------------- |
| `pnpm start:live`                         | <https://crates.io>                       | Testing UI changes with the full live site's data       |
| `pnpm start:staging`                      | <https://staging-crates-io.herokuapp.com> | Testing UI changes with a smaller set of realistic data |
| `pnpm start:local`                        | Backend server running locally            | See the Working on the backend section for setup        |
| `pnpm start -- --proxy https://crates.io` | Whatever is specified in `--proxy` arg    | If your use case is not covered here                    |

### Running the frontend tests

You can run the frontend tests with:

```console
pnpm test
```

## Working on the Backend

### Backend Requirements

In order to run the backend, you will need to have installed:

- [Rust](https://www.rust-lang.org/en-US/) stable (see `rust-toolchain.toml` for version) and cargo, which comes with Rust
- [Postgres](https://www.postgresql.org/) >= 9.5
- [OpenSSL](https://www.openssl.org/) >= 1.0.2k
- [diesel_cli](http://diesel.rs/guides/getting-started/) >= 2.0.0 and < 3.0.0

> [!NOTE]
> The backend codebase assumes `cfg(unix)`. If you're running on Windows we recommend that you use
> a WSL environment for development and follow the Linux instructions below.

#### Rust

- [rustup](https://rustup.rs/) is the installation method we'd recommend for
  all platforms.

#### Postgres

Postgres can be a little finicky to install and get set up. These are the
methods we'd recommend for each operating system:

- macOS: Either [Postgres.app](https://postgresapp.com/) or through
  [Homebrew](https://brew.sh/) by running `brew install postgresql@13` and
  following the post-installation instructions
- Linux: Postgres is generally available in the distribution repositories as
  `postgresql` or `postgresql-server`. You will also need the developer package
  (known either as `postgresql-devel` or `libpq-dev`),
  as well as `postgresql-contrib`. Here
  are some examples of installation commands that have been tested for the
  following distributions:
  - Ubuntu: `sudo apt-get install postgresql postgresql-contrib libpq-dev pkg-config`
  - Fedora: `sudo dnf install postgresql-server postgresql-contrib postgresql-devel pkgconfig`

  > If you're missing a package, when you try to `cargo install` or `cargo
build` later, you'll get an error that looks like this:
  >
  > ```text
  > error: linking with `cc` failed: exit code: 1
  > [lots of output]
  > = note: /usr/bin/ld: cannot find -l[something]
  > ```
  >
  > That `[something]` is what you're missing; you'll need to do some research
  > to figure out what package will get you the missing library.

Then, once Postgres is installed, ensure that you can run `psql` (and then exit
by typing `\q`) without any errors to connect to your running Postgres server.

> If you see an error that looks like this:
>
> ```text
> psql: could not connect to server: No such file or directory
> Is the server running locally and accepting
> connections on Unix domain socket "/var/run/postgresql/.s.PGSQL.5432"?
> ```
>
> You may need to start the postgreql server on your system. On a Linux system,
> you can start it with this command:
>
> ```console
> sudo service postgresql start
> ```
>
> Depending on your system, its permissions, and how Postgres was installed, you
> may need to use the `postgres` user for some operations (by using `sudo su -
postgres`). Generally, the problem is that by default the postgres server is
> only set up to allow connections by the `postrges` user. You'll know if you're
> in this situation because if you try to run `psql` as yourself, you'll get
> this error:
>
> ```text
> psql: FATAL:  role "yourusername" does not exist
> ```
>
> One way of fixing this is to first give yourself superuser permissions in the
> database by running this and replacing `[yourusername]` with your username:
>
> ```console
> sudo -u postgres createuser --superuser [yourusername]
> ```
>
> Next, if you try to run `psql` and get this error:
>
> ```text
> psql: FATAL:  database "yourusername" does not exist
> ```
>
> Fix that by creating a template database for yourself:
>
> ```console
> createdb [yourusername]
> ```
>
> Try running `psql` again as yourself. If you're still getting errors, here are
> some pages with troubleshooting information for some of the Linux
> distributions:
>
> - [Ubuntu](https://help.ubuntu.com/community/PostgreSQL)
> - [Fedora](https://fedoraproject.org/wiki/PostgreSQL)
>
> For other platforms, try searching for the error message and following
> suggestions from Stack Overflow. Open an issue on this repo if you get stuck,
> we'll help fix the problem and will add the solution to these
> instructions!

Another option is to use a standalone Docker container for Postgres:

```sh
# example using postgres 16
docker run -e POSTGRES_PASSWORD=password -p 5432:5432 postgres:16
# database URL will be
# DATABASE_URL=postgres://postgres:password@localhost:5432/cargo_registry
```

#### OpenSSL

- macOS: you can also install with homebrew by using `brew install openssl`
- Linux: you should be able to use the distribution repositories. It will be
  called `openssl`, `openssl-devel`, or `libssl-dev`. OpenSSL needs
  `pkg-config` as well. According to
  [rust-openssl](https://github.com/sfackler/rust-openssl),
  - Ubuntu: `sudo apt-get install libssl-dev pkg-config`
  - Fedora: `sudo dnf install openssl-devel pkgconfig`
  - Arch Linux: `sudo pacman -S openssl pkg-config`

> [!TIP]
> If you have problems with OpenSSL, see [rust-openssl's
> README](https://github.com/sfackler/rust-openssl) for some suggestions.

#### `diesel_cli`

On all platforms, install through `cargo` by running:

```console
cargo install diesel_cli --no-default-features --features postgres --version ^2
```

This will install a binary named `diesel`, so you should be able to run `diesel
--version` to confirm successful installation.

> If you're on Linux and this fails with an error that looks like ``error:
linking with `cc` failed: exit code: 1``, you're probably missing some
> Postgres related libraries. See the Postgres section above on how to fix this.

### Building and serving the backend

#### Environment variables

Copy the `.env.sample` file to `.env` and then modify `.env` as appropriate. On
Unix systems, the default configuration will use a local Unix socket which does
not require setting a password for the database user.

On other platforms, or if connecting to your database over IP:

> Change this by filling in this template with the
> appropriate values where there are `[]`s:
>
> ```text
> postgres://[postgresuser]:[password]@[localhost]:[5432]/[database_name]
> ```
>
> - Replace `[postgresuser]` with the user that is allowed to log in to your
>   Postgres server.
> - If that user needs a password, put it after the `:` instead of
>   `[password]`. Remove both `:` and `[password]` if the user doesn't need a
>   password.
> - Replace `[localhost]` with the host and `[5432]` with the port where your
>   Postgres server is running.
> - Replace `[database_name]` with the name of the database you'd like to use.
>   We're going to create a database named `cargo_registry` in the next
>   section; change this if you'd like to name it something else.
>
> If you receive an error that looks like:
>
> ```text
> password authentication failed for user \"postgres\"\nFATAL:
> password authentication failed for user \"postgres\"\n"`
> ```
>
> You may need to update the pg_hba.conf file on your development workstation.
> For a guide to finding your pg_hba.conf file, check out [this post](https://askubuntu.com/questions/256534/how-do-i-find-the-path-to-pg-hba-conf-from-the-shell) on the Ubuntu Stack Exchange.
> For information on updating your pg_hba.conf file and reloading it, see [this post](https://stackoverflow.com/questions/17996957/fe-sendauth-no-password-supplied) on Stack Overflow.

#### Creating the database

You can name your development database anything as long as it matches the
database name in the `DATABASE_URL` value. This example assumes a database
named `cargo_registry`.

Create a new database by running:

```console
createdb cargo_registry
```

Then run the migrations:

```console
diesel migration run
```

#### Setting up the git index

Set up the git repo for the crate index by running:

```console
./script/init-local-index.sh
```

#### Importing a database dump

You can then import the database with

```console
./script/import-database-dump.sh
```

#### Starting the server and the frontend

Build and start the server by running this command (you'll need to stop this
with `CTRL-C` and rerun this command every time you change the backend code):

```console
cargo run
```

Then start the background worker (which will process uploaded READMEs):

```console
cargo run --bin background-worker
```

Since crates.io is using the `tracing` crate, you can enable debug logging by
setting the `RUST_LOG` environment variable to `debug` before running them, for
example:

```console
RUST_LOG=debug cargo run --bin background-worker
```
Then start a frontend that uses this backend by running this command in another
terminal session (the frontend picks up frontend changes using live reload
without a restart needed, and you can leave the frontend running while you
restart the server):

```console
pnpm start:local
```

And then you should be able to visit <http://localhost:4200>!

#### Using Mailgun to Send Emails

We currently have email functionality enabled for confirming a user's email
address. In development, the sending of emails is simulated by a file
representing the email being created in your local `/tmp/` directory
(If you are using docker, it is in the `/tmp/` directory of the backend container).
The file looks like:

```eml
To: rustin.liu@gmail.com
From: test@localhost
Subject: Please confirm your email address
Content-Transfer-Encoding: 7bit
Date: Thu, 24 Jun 2021 08:02:23 -0000

Hello hi-rustin! Welcome to crates.io. Please click the
link below to verify your email address. Thank you!

https://crates.io/confirm/RiphVyFo31wuKQhpyTw7RF2LIf
```

When verifying the email, you need to change the prefix to your frontend host.
For example, change the above link to `http://localhost:4200/confirm/RiphVyFo31wuKQhpyTw7RF2LIf`.

If you want to test sending real emails, you will have to either set the
Mailgun environment variables in `.env` manually or run your app instance
on Heroku and add the Mailgun app.

To set the environment variables manually, create an account and configure
Mailgun. [These quick start instructions](https://documentation.mailgun.com/en/latest/quickstart.html)
might be helpful. Once you get the environment variables for the app, you
will have to add them to the bottom of the `.env` file. You will need to
fill in the `MAILGUN_SMTP_LOGIN`, `MAILGUN_SMTP_PASSWORD`, and
`MAILGUN_SMTP_SERVER` fields.

If using Heroku, you should be able to add the app to your instance on your
dashboard. When your code is pushed and run on Heroku, the environment
variables should be detected and you should not have to set anything
manually.

In either case, you should be able to check in your Mailgun account to see
if emails are being detected and sent. Relevant information should be under
the 'logs' tab on your Mailgun dashboard. To access, if the variables were
set up manually, log in to your account. If the variables were set through
Heroku, you should be able to click on the Mailgun icon in your Heroku
dashboard, which should take you to your Mailgun dashboard.

### Running the backend tests

In your `.env` file, set `TEST_DATABASE_URL` to a value that's the same as
`DATABASE_URL`, or use a different database name. The `TEST_DATABASE_URL`
connection will be used to create new databases for the tests, with names
prefixed with the database name from `TEST_DATABASE_URL`.

Example: `postgres:///cargo_registry_test`.

Create the test database by running:

```console
createdb cargo_registry_test
```

The test harness will ensure that migrations are run.

Run the backend API server tests with this command:

```console
cargo test
```

### Using your local crates.io with cargo

Once you have a local instance of crates.io running at <http://localhost:4200> by
following the instructions in the "Working on the Backend" section, you can go
to another Rust project and tell cargo to use your local crates.io instead of
production.

#### Publishing a crate to your local crates.io

In order to publish a crate, you need an API token. In order to get an API
token, you need to be able to log in with GitHub OAuth. In order to be able to
log in with GitHub, you need to create an application with GitHub and specify
the `GH_CLIENT_ID` and `GH_CLIENT_SECRET` variables in your `.env`.

To create an application with GitHub, go to [Settings -> Developer Settings ->
OAuth Applications](https://github.com/settings/developers) and click on the
"Register a new application" button. Fill in the form as follows:

- Application name: name your application whatever you'd like.
- Homepage URL: `http://localhost:4200/`
- Authorization callback URL: `http://localhost:4200/github-redirect.html`

Create the application, then take the Client ID ad Client Secret values and use
them as the values of the `GH_CLIENT_ID` and `GH_CLIENT_SECRET` in your `.env`.

Then restart your backend, and you should be able to log in to your local
crates.io with your GitHub account.

Go to <http://localhost:4200/me> to get your API token and run the `cargo login`
command as directed.

Now you should be able to go to the directory of a crate that has no
dependencies. There's currently restrictions around publishing crates that have
dependencies installed from other registries than the one you're publishing to,
so if you have a crate you've been working on that has dependencies from the
live crates.io, you won't be able to publish that crate locally.

In your crate directory, run:

```console
cargo publish --index file:///path/to/your/crates.io/tmp/index-bare --token $YOUR_TOKEN
```

> If you're using an older version of cargo you should use `--host` instead of `--index`.

where `file:///path/to/your/crates.io` is the directory that you have
crates.io's code in, and `tmp/index-bare` is the directory with the git index
that `./script/init-local-index.sh` set up.

Note that when you're running crates.io in development mode without the S3
variables set (which is what we've done in these setup steps), the crate files
will be stored in `local_uploads/crates` and served from there when a
crate is downloaded. If you try to install a crate from your local crates.io and
`cargo` can't find the crate files, it is probably because this directory does not
exist.

#### Downloading a crate from your local crates.io

In _another_ crate, you can use the crate you've published as a dependency by
telling `cargo` to replace crates.io with your local crates.io as a source.

In this other crate's directory, create a `.cargo/config` file with this
content:

```toml
[source]

[source.mirror]
registry = "file:///path/to/your/crates.io/tmp/index-bare"

[source.crates-io]
replace-with = "mirror"
```

Then add the crate you published to your local crates.io as a dependency in
this crate's `Cargo.toml`, and `cargo build` should display output like this:

```console
    Updating registry `file:///path/to/your/crates.io/tmp/index-bare`
 Downloading yourcrate v0.1.0 (registry file:///path/to/your/crates.io/tmp/index-bare)
   Compiling yourcrate v0.1.0
   Compiling thiscrate v0.1.0 (file:///path/to/thiscrate)
    Finished dev [unoptimized + debuginfo] target(s) in 0.56 secs
```

## Running crates.io with Docker

There are Dockerfiles to build both the backend and the frontend,
(`backend.Dockerfile` and `frontend.Dockerfile`) respectively, but it is most
useful to just use docker-compose to bring up everything that's needed all in
one go:

```console
docker compose up -d
```

The Compose file is filled out with a sane set of defaults that should Just
Work™ out of the box without any modification. Individual settings can be
overridden by creating a `docker-compose.override.yml` with the updated config.
For example, in order to specify a set of GitHub OAuth Client credentials, a
`docker-compose.override.yml` file might look like this:

```yaml
services:
  backend:
    environment:
      GH_CLIENT_ID: blahblah_ID
      GH_CLIENT_SECRET: blahblah_secret
```

These environment variables can also be defined in a local `.env` file, see `.env.sample`
for various configuration options.

### Accessing services

By default, the services will be exposed on their normal ports:

- `5432` for Postgres
- `8888` for the crates.io backend
- `4200` for the crates.io frontend

These can be changed with the `docker-compose.override.yml` file.

### Publishing crates

Unlike a local setup, the Git index is not stored in the `./tmp` folder, so in
order to publish to the Dockerized crates.io, run

```console
cargo publish --index http://localhost:8888/git/index --token $YOUR_TOKEN
```

### Changing code

The `app/` directory is mounted directly into the frontend Docker container,
which means that the Ember live-reload server will still just work. If
anything outside of `app/` is changed, the base Docker image will have to be
rebuilt:

```console
# Rebuild frontend Docker image
docker compose build frontend

# Restart running frontend container (if it's already running)
docker compose stop frontend
docker compose rm frontend
docker compose up -d
```

Similarly, the `src/` directory is mounted into the backend Docker container,
so in order to recompile the backend, run:

```console
docker compose restart backend
```

If anything outside of `src/` is changed, the base Docker image will have to be
rebuilt:

```console
# Rebuild backend Docker image
docker compose build backend

# Restart running backend container (if it's already running)
docker compose stop backend
docker compose rm backend
docker compose up -d
```

### Volumes

A number of names volumes are created, as can be seen in the `volumes` section
of the `docker-compose.yml` file.
