import { assert, test } from 'vitest';

import { db } from '../../../index.js';

test('happy path', async function () {
  let crate = db.crate.create({ name: 'test-crate' });
  db.version.create({ crate });

  let user = db.user.create({ email_verified: true });
  db.mswSession.create({ user });

  // Create crate ownership
  db.crateOwnership.create({
    crate,
    user,
  });

  // Create GitLab configs
  let config1 = db.trustpubGitlabConfig.create({
    crate,
    namespace: 'rust-lang',
    namespace_id: null,
    project: 'crates.io',
    workflow_filepath: '.gitlab-ci.yml',
    created_at: '2023-01-01T00:00:00Z',
  });

  let config2 = db.trustpubGitlabConfig.create({
    crate,
    namespace: 'rust-lang',
    namespace_id: '12345',
    project: 'cargo',
    workflow_filepath: '.gitlab/ci.yml',
    environment: 'production',
    created_at: '2023-02-01T00:00:00Z',
  });

  let response = await fetch(`/api/v1/trusted_publishing/gitlab_configs?crate=${crate.name}`);
  assert.strictEqual(response.status, 200);
  assert.deepEqual(await response.json(), {
    gitlab_configs: [
      {
        id: Number(config1.id),
        crate: crate.name,
        namespace: 'rust-lang',
        namespace_id: null,
        project: 'crates.io',
        workflow_filepath: '.gitlab-ci.yml',
        environment: null,
        created_at: '2023-01-01T00:00:00Z',
      },
      {
        id: Number(config2.id),
        crate: crate.name,
        namespace: 'rust-lang',
        namespace_id: '12345',
        project: 'cargo',
        workflow_filepath: '.gitlab/ci.yml',
        environment: 'production',
        created_at: '2023-02-01T00:00:00Z',
      },
    ],
  });
});

test('happy path with no configs', async function () {
  let crate = db.crate.create({ name: 'test-crate-empty' });
  db.version.create({ crate });

  let user = db.user.create({ email_verified: true });
  db.mswSession.create({ user });

  // Create crate ownership
  db.crateOwnership.create({
    crate,
    user,
  });

  let response = await fetch(`/api/v1/trusted_publishing/gitlab_configs?crate=${crate.name}`);
  assert.strictEqual(response.status, 200);
  assert.deepEqual(await response.json(), {
    gitlab_configs: [],
  });
});

test('returns 403 if unauthenticated', async function () {
  let response = await fetch(`/api/v1/trusted_publishing/gitlab_configs?crate=test-crate`);
  assert.strictEqual(response.status, 403);
  assert.deepEqual(await response.json(), {
    errors: [{ detail: 'must be logged in to perform that action' }],
  });
});

test('returns 400 if query params are missing', async function () {
  let user = db.user.create();
  db.mswSession.create({ user });

  let response = await fetch(`/api/v1/trusted_publishing/gitlab_configs`);
  assert.strictEqual(response.status, 400);
  assert.deepEqual(await response.json(), {
    errors: [{ detail: 'missing or invalid filter' }],
  });
});

test("returns 404 if crate can't be found", async function () {
  let user = db.user.create();
  db.mswSession.create({ user });

  let response = await fetch(`/api/v1/trusted_publishing/gitlab_configs?crate=nonexistent`);
  assert.strictEqual(response.status, 404);
  assert.deepEqual(await response.json(), {
    errors: [{ detail: 'Not Found' }],
  });
});

test('returns 400 if user is not an owner of the crate', async function () {
  let crate = db.crate.create({ name: 'test-crate-not-owner' });
  db.version.create({ crate });

  let user = db.user.create();
  db.mswSession.create({ user });

  let response = await fetch(`/api/v1/trusted_publishing/gitlab_configs?crate=${crate.name}`);
  assert.strictEqual(response.status, 400);
  assert.deepEqual(await response.json(), {
    errors: [{ detail: 'You are not an owner of this crate' }],
  });
});
