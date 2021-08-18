# Deployment Instructions

## Heroku

crates.io runs on the [Heroku] "Cloud Application Platform". For security
reasons only a very small number of people have access to the production app
on Heroku. If you don't have access, but need something to be deployed let us
know on [Discord](https://discord.gg/rust-lang) in the `#crates-io-team`
channel.

[Heroku]: https://heroku.com/

For deployments, you will need to install the [Heroku CLI] application. Once
installed, you need to run `heroku login`, so that the CLI has access to your
Heroku account.

[Heroku CLI]: https://devcenter.heroku.com/articles/heroku-cli

Once you are logged in, you can run the following commands to add the staging
and production app as git remotes to your local clone of the crates.io
repository:

```sh
heroku git:remote -a staging-crates-io -r heroku-staging
heroku git:remote -a crates-io -r heroku-prod
```

*These steps above only need to be done once, not for every deployment.*

## Update your local crates.io clone

```sh
# fetch updates from https://github.com/rust-lang/crates.io
git fetch upstream

# switch to the `master` branch
git checkout master

# reset the local `master` branch to the state of https://github.com/rust-lang/crates.io
git reset --hard upstream/master
```

## Check the diff

You can run `./scripts/create-diff-link.sh` to generate a GitHub diff link that
compares the current state of your local `master` branch (which should match
the upstream state; see above) and the current revision that is running in
production.

Make sure that the diff does not contain any surprises or changes you are unsure
about. If you have doubt, ask the other team members on Discord.

## Announce the deployment

To avoid conflicting deployments you should announce your deployment on the
`#crates-io-operations` Discord channel. Example:

> Deploying `3c448add..86ded9db` to staging and production. This adds the
> `DELETE /tokens/current` endpoint, updates some frontend dependencies, and
> drops the `request_id` from successful download requests to reduce our log
> traffic in order to stay under our current plan capacity. This is safe to
> rollback. https://github.com/rust-lang/crates.io/compare/3c448add...86ded9db

In case the deployment contains database changes that are **not** safe to
rollback, make sure to explicitly highlight it.

## Deploy to the staging environment

Before deploying to production, it is recommended to deploy to our staging
environment first and check that everything still works as intended:

```sh
git push heroku-staging master
```

Get a coffee… or tea… this will take a bit. Once the deployment is complete, you
should see something like this:

```
-----> Launching...
 !     Release command declared: this new release will not be available until the command succeeds.
       Released v634
       https://staging-crates-io.herokuapp.com/ deployed to Heroku
```

Afterwards, you can visit <https://staging.crates.io> to check if the deployed
changes work as intended and nothing broke.

## Deploy to production

After all the above steps are successfully completed we can deploy to
production, similar to how we deployed to the staging environment:

```sh
git push heroku-prod master
```

This step will also take a while, but at the end you should see a similar
message as above, and now it's time to verify on <https://crates.io> that
nothing has broken.

## Rollbacks

If something has broken, the deployment should be rolled back. You can use the
"Roll back to here" links on <https://dashboard.heroku.com/apps/crates-io/activity>
for this purpose, but be aware that database migration rollbacks need additional
work.
