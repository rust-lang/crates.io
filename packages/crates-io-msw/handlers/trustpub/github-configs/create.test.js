import { afterEach, assert, beforeEach, test, vi } from 'vitest';

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

  let response = await fetch('/api/v1/trusted_publishing/github_configs', {
    method: 'POST',
    body: JSON.stringify({
      github_config: {
        crate: crate.name,
        repository_owner: 'rust-lang',
        repository_name: 'crates.io',
        workflow_filename: 'ci.yml',
      },
    }),
  });

  assert.strictEqual(response.status, 200);
  assert.deepEqual(await response.json(), {
    github_config: {
      id: 1,
      crate: crate.name,
      repository_owner: 'rust-lang',
      repository_owner_id: 5_430_905,
      repository_name: 'crates.io',
      workflow_filename: 'ci.yml',
      environment: null,
      created_at: '2023-01-01T00:00:00.000Z',
    },
  });
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

  let response = await fetch('/api/v1/trusted_publishing/github_configs', {
    method: 'POST',
    body: JSON.stringify({
      github_config: {
        crate: crate.name,
        repository_owner: 'rust-lang',
        repository_name: 'crates.io',
        workflow_filename: 'ci.yml',
        environment: 'production',
      },
    }),
  });

  assert.strictEqual(response.status, 200);
  assert.deepEqual(await response.json(), {
    github_config: {
      id: 1,
      crate: crate.name,
      repository_owner: 'rust-lang',
      repository_owner_id: 5_430_905,
      repository_name: 'crates.io',
      workflow_filename: 'ci.yml',
      environment: 'production',
      created_at: '2023-02-01T00:00:00.000Z',
    },
  });
});

test('returns 403 if unauthenticated', async function () {
  let response = await fetch('/api/v1/trusted_publishing/github_configs', {
    method: 'POST',
    body: JSON.stringify({
      github_config: {
        crate: 'test-crate',
        repository_owner: 'rust-lang',
        repository_name: 'crates.io',
        workflow_filename: 'ci.yml',
      },
    }),
  });

  assert.strictEqual(response.status, 403);
  assert.deepEqual(await response.json(), {
    errors: [{ detail: 'must be logged in to perform that action' }],
  });
});

test('returns 400 if request body is invalid', async function () {
  let user = await db.user.create();
  await db.mswSession.create({ user });

  let response = await fetch('/api/v1/trusted_publishing/github_configs', {
    method: 'POST',
    body: JSON.stringify({}),
  });

  assert.strictEqual(response.status, 400);
  assert.deepEqual(await response.json(), {
    errors: [{ detail: 'invalid request body' }],
  });
});

test('returns 400 if required fields are missing', async function () {
  let user = await db.user.create();
  await db.mswSession.create({ user });

  let response = await fetch('/api/v1/trusted_publishing/github_configs', {
    method: 'POST',
    body: JSON.stringify({
      github_config: {
        crate: 'test-crate',
      },
    }),
  });

  assert.strictEqual(response.status, 400);
  assert.deepEqual(await response.json(), {
    errors: [{ detail: 'missing required fields' }],
  });
});

test("returns 404 if crate can't be found", async function () {
  let user = await db.user.create();
  await db.mswSession.create({ user });

  let response = await fetch('/api/v1/trusted_publishing/github_configs', {
    method: 'POST',
    body: JSON.stringify({
      github_config: {
        crate: 'nonexistent',
        repository_owner: 'rust-lang',
        repository_name: 'crates.io',
        workflow_filename: 'ci.yml',
      },
    }),
  });

  assert.strictEqual(response.status, 404);
  assert.deepEqual(await response.json(), {
    errors: [{ detail: 'Not Found' }],
  });
});

test('returns 400 if user is not an owner of the crate', async function () {
  let crate = await db.crate.create({ name: 'test-crate-not-owner' });
  await db.version.create({ crate });

  let user = await db.user.create();
  await db.mswSession.create({ user });

  let response = await fetch('/api/v1/trusted_publishing/github_configs', {
    method: 'POST',
    body: JSON.stringify({
      github_config: {
        crate: crate.name,
        repository_owner: 'rust-lang',
        repository_name: 'crates.io',
        workflow_filename: 'ci.yml',
      },
    }),
  });

  assert.strictEqual(response.status, 400);
  assert.deepEqual(await response.json(), {
    errors: [{ detail: 'You are not an owner of this crate' }],
  });
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

  let response = await fetch('/api/v1/trusted_publishing/github_configs', {
    method: 'POST',
    body: JSON.stringify({
      github_config: {
        crate: crate.name,
        repository_owner: 'rust-lang',
        repository_name: 'crates.io',
        workflow_filename: 'ci.yml',
      },
    }),
  });

  assert.strictEqual(response.status, 403);
  assert.deepEqual(await response.json(), {
    errors: [{ detail: 'You must verify your email address to create a Trusted Publishing config' }],
  });
});
