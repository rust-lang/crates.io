import { expect, test } from 'vitest';

import { db } from '../../../index.js';

test('happy path', async function () {
  let crate = await db.crate.create({ name: 'test-crate' });
  await db.version.create({ crate });

  let user = await db.user.create({ email_verified: true });
  await db.mswSession.create({ user });

  // Create crate ownership
  await db.crateOwnership.create({
    crate,
    user,
  });

  // Create GitHub configs
  await db.trustpubGithubConfig.create({
    crate,
    repository_owner: 'rust-lang',
    repository_owner_id: 1,
    repository_name: 'crates.io',
    workflow_filename: 'ci.yml',
    created_at: '2023-01-01T00:00:00Z',
  });

  await db.trustpubGithubConfig.create({
    crate,
    repository_owner: 'rust-lang',
    repository_owner_id: 42,
    repository_name: 'cargo',
    workflow_filename: 'release.yml',
    environment: 'production',
    created_at: '2023-02-01T00:00:00Z',
  });

  let response = await fetch(`/api/v1/trusted_publishing/github_configs?crate=${crate.name}`);
  expect(response.status).toBe(200);
  expect(await response.json()).toMatchInlineSnapshot(`
    {
      "github_configs": [
        {
          "crate": "test-crate",
          "created_at": "2023-01-01T00:00:00Z",
          "environment": null,
          "id": 1,
          "repository_name": "crates.io",
          "repository_owner": "rust-lang",
          "repository_owner_id": 1,
          "workflow_filename": "ci.yml",
        },
        {
          "crate": "test-crate",
          "created_at": "2023-02-01T00:00:00Z",
          "environment": "production",
          "id": 2,
          "repository_name": "cargo",
          "repository_owner": "rust-lang",
          "repository_owner_id": 42,
          "workflow_filename": "release.yml",
        },
      ],
    }
  `);
});

test('happy path with no configs', async function () {
  let crate = await db.crate.create({ name: 'test-crate-empty' });
  await db.version.create({ crate });

  let user = await db.user.create({ email_verified: true });
  await db.mswSession.create({ user });

  // Create crate ownership
  await db.crateOwnership.create({
    crate,
    user,
  });

  let response = await fetch(`/api/v1/trusted_publishing/github_configs?crate=${crate.name}`);
  expect(response.status).toBe(200);
  expect(await response.json()).toMatchInlineSnapshot(`
    {
      "github_configs": [],
    }
  `);
});

test('returns 403 if unauthenticated', async function () {
  let response = await fetch(`/api/v1/trusted_publishing/github_configs?crate=test-crate`);
  expect(response.status).toBe(403);
  expect(await response.json()).toMatchInlineSnapshot(`
    {
      "errors": [
        {
          "detail": "must be logged in to perform that action",
        },
      ],
    }
  `);
});

test('returns 400 if query params are missing', async function () {
  let user = await db.user.create();
  await db.mswSession.create({ user });

  let response = await fetch(`/api/v1/trusted_publishing/github_configs`);
  expect(response.status).toBe(400);
  expect(await response.json()).toMatchInlineSnapshot(`
    {
      "errors": [
        {
          "detail": "missing or invalid filter",
        },
      ],
    }
  `);
});

test("returns 404 if crate can't be found", async function () {
  let user = await db.user.create();
  await db.mswSession.create({ user });

  let response = await fetch(`/api/v1/trusted_publishing/github_configs?crate=nonexistent`);
  expect(response.status).toBe(404);
  expect(await response.json()).toMatchInlineSnapshot(`
    {
      "errors": [
        {
          "detail": "Not Found",
        },
      ],
    }
  `);
});

test('returns 400 if user is not an owner of the crate', async function () {
  let crate = await db.crate.create({ name: 'test-crate-not-owner' });
  await db.version.create({ crate });

  let user = await db.user.create();
  await db.mswSession.create({ user });

  let response = await fetch(`/api/v1/trusted_publishing/github_configs?crate=${crate.name}`);
  expect(response.status).toBe(400);
  expect(await response.json()).toMatchInlineSnapshot(`
    {
      "errors": [
        {
          "detail": "You are not an owner of this crate",
        },
      ],
    }
  `);
});
