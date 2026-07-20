import { afterEach, beforeEach, expect, test, vi } from 'vitest';

import { db } from '../../../index.js';

beforeEach(() => {
  vi.useFakeTimers();
});

afterEach(() => {
  vi.restoreAllMocks();
});

test('happy path', async function () {
  vi.setSystemTime(new Date('2023-01-01T00:00:00Z'));

  let crate = await db.crate.create({ name: 'test-crate' });
  await db.version.create({ crate });

  let user = await db.user.create({ emailVerified: true });
  await db.mswSession.create({ user });

  // Create crate ownership
  await db.crateOwnership.create({
    crate,
    user,
  });

  let response = await fetch('/api/v1/trusted_publishing/gitlab_configs', {
    method: 'POST',
    body: JSON.stringify({
      gitlab_config: {
        crate: crate.name,
        namespace: 'rust-lang',
        project: 'crates.io',
        workflow_filepath: '.gitlab-ci.yml',
      },
    }),
  });

  expect(response.status).toBe(200);
  expect(await response.json()).toMatchInlineSnapshot(`
    {
      "gitlab_config": {
        "crate": "test-crate",
        "created_at": "2023-01-01T00:00:00.000Z",
        "environment": null,
        "id": 1,
        "namespace": "rust-lang",
        "namespace_id": null,
        "project": "crates.io",
        "workflow_filepath": ".gitlab-ci.yml",
      },
    }
  `);
});

test('happy path with environment', async function () {
  vi.setSystemTime(new Date('2023-02-01T00:00:00Z'));

  let crate = await db.crate.create({ name: 'test-crate-env' });
  await db.version.create({ crate });

  let user = await db.user.create({ emailVerified: true });
  await db.mswSession.create({ user });

  // Create crate ownership
  await db.crateOwnership.create({
    crate,
    user,
  });

  let response = await fetch('/api/v1/trusted_publishing/gitlab_configs', {
    method: 'POST',
    body: JSON.stringify({
      gitlab_config: {
        crate: crate.name,
        namespace: 'rust-lang',
        project: 'crates.io',
        workflow_filepath: '.gitlab-ci.yml',
        environment: 'production',
      },
    }),
  });

  expect(response.status).toBe(200);
  expect(await response.json()).toMatchInlineSnapshot(`
    {
      "gitlab_config": {
        "crate": "test-crate-env",
        "created_at": "2023-02-01T00:00:00.000Z",
        "environment": "production",
        "id": 1,
        "namespace": "rust-lang",
        "namespace_id": null,
        "project": "crates.io",
        "workflow_filepath": ".gitlab-ci.yml",
      },
    }
  `);
});

test('returns 403 if unauthenticated', async function () {
  let response = await fetch('/api/v1/trusted_publishing/gitlab_configs', {
    method: 'POST',
    body: JSON.stringify({
      gitlab_config: {
        crate: 'test-crate',
        namespace: 'rust-lang',
        project: 'crates.io',
        workflow_filepath: '.gitlab-ci.yml',
      },
    }),
  });

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

test('returns 400 if request body is invalid', async function () {
  let user = await db.user.create({});
  await db.mswSession.create({ user });

  let response = await fetch('/api/v1/trusted_publishing/gitlab_configs', {
    method: 'POST',
    body: JSON.stringify({}),
  });

  expect(response.status).toBe(400);
  expect(await response.json()).toMatchInlineSnapshot(`
    {
      "errors": [
        {
          "detail": "invalid request body",
        },
      ],
    }
  `);
});

test('returns 400 if required fields are missing', async function () {
  let user = await db.user.create({});
  await db.mswSession.create({ user });

  let response = await fetch('/api/v1/trusted_publishing/gitlab_configs', {
    method: 'POST',
    body: JSON.stringify({
      gitlab_config: {
        crate: 'test-crate',
      },
    }),
  });

  expect(response.status).toBe(400);
  expect(await response.json()).toMatchInlineSnapshot(`
    {
      "errors": [
        {
          "detail": "missing required fields",
        },
      ],
    }
  `);
});

test("returns 404 if crate can't be found", async function () {
  let user = await db.user.create({});
  await db.mswSession.create({ user });

  let response = await fetch('/api/v1/trusted_publishing/gitlab_configs', {
    method: 'POST',
    body: JSON.stringify({
      gitlab_config: {
        crate: 'nonexistent',
        namespace: 'rust-lang',
        project: 'crates.io',
        workflow_filepath: '.gitlab-ci.yml',
      },
    }),
  });

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

  let user = await db.user.create({});
  await db.mswSession.create({ user });

  let response = await fetch('/api/v1/trusted_publishing/gitlab_configs', {
    method: 'POST',
    body: JSON.stringify({
      gitlab_config: {
        crate: crate.name,
        namespace: 'rust-lang',
        project: 'crates.io',
        workflow_filepath: '.gitlab-ci.yml',
      },
    }),
  });

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

test('returns 403 if user email is not verified', async function () {
  let crate = await db.crate.create({ name: 'test-crate-unverified' });
  await db.version.create({ crate });

  let user = await db.user.create({ emailVerified: false });
  await db.mswSession.create({ user });

  // Create crate ownership
  await db.crateOwnership.create({
    crate,
    user,
  });

  let response = await fetch('/api/v1/trusted_publishing/gitlab_configs', {
    method: 'POST',
    body: JSON.stringify({
      gitlab_config: {
        crate: crate.name,
        namespace: 'rust-lang',
        project: 'crates.io',
        workflow_filepath: '.gitlab-ci.yml',
      },
    }),
  });

  expect(response.status).toBe(403);
  expect(await response.json()).toMatchInlineSnapshot(`
    {
      "errors": [
        {
          "detail": "You must verify your email address to create a Trusted Publishing config",
        },
      ],
    }
  `);
});
