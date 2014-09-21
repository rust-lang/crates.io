# Cargo

This README outlines the details of collaborating on this Ember application.

## Installation

* `git clone` this repository
* `npm install`
* `bower install`

## Configuration

* `export S3_BUCKET=`
* `export S3_ACCESS_KEY=`
* `export S3_SECRET_KEY=`
* `export SESSION_KEY=`
* `export GH_CLIENT_ID=`
* `export GH_CLIENT_SECRET=`
* `export DATABASE_URL=`

* `export GIT_REPO_URL=<GH repo url>`
* `export GIT_REPO_CHECKOUT=<path/to/gh/checkout>`

## Running

* `cargo build`
* `./target/cargo-registry`
* `ember server --proxy http://localhost:8888`
* Visit your app at [http://localhost:4200](http://localhost:4200).

## Running Tests

* `ember test`
* `ember test --server`

## Building

* `cargo build`
* `ember build`

For more information on using ember-cli, visit [http://iamstef.net/ember-cli/](http://iamstef.net/ember-cli/).
