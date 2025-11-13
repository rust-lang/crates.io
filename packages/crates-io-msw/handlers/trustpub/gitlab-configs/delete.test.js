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

  // Create GitLab config
  let config = db.trustpubGitlabConfig.create({
    crate,
    namespace: 'rust-lang',
    project: 'crates.io',
    workflow_filepath: '.gitlab-ci.yml',
    created_at: '2023-01-01T00:00:00Z',
  });

  let response = await fetch(`/api/v1/trusted_publishing/gitlab_configs/${config.id}`, {
    method: 'DELETE',
  });

  assert.strictEqual(response.status, 204);
  assert.strictEqual(await response.text(), '');

  // Verify the config was deleted
  let deletedConfig = db.trustpubGitlabConfig.findFirst({ where: { id: { equals: config.id } } });
  assert.strictEqual(deletedConfig, null);
});

test('returns 403 if unauthenticated', async function () {
  let response = await fetch('/api/v1/trusted_publishing/gitlab_configs/1', {
    method: 'DELETE',
  });

  assert.strictEqual(response.status, 403);
  assert.deepEqual(await response.json(), {
    errors: [{ detail: 'must be logged in to perform that action' }],
  });
});

test('returns 404 if config ID is invalid', async function () {
  let user = db.user.create();
  db.mswSession.create({ user });

  let response = await fetch('/api/v1/trusted_publishing/gitlab_configs/invalid', {
    method: 'DELETE',
  });

  assert.strictEqual(response.status, 404);
  assert.deepEqual(await response.json(), {
    errors: [{ detail: 'Not Found' }],
  });
});

test("returns 404 if config can't be found", async function () {
  let user = db.user.create();
  db.mswSession.create({ user });

  let response = await fetch('/api/v1/trusted_publishing/gitlab_configs/999999', {
    method: 'DELETE',
  });

  assert.strictEqual(response.status, 404);
  assert.deepEqual(await response.json(), {
    errors: [{ detail: 'Not Found' }],
  });
});

test('returns 400 if user is not an owner of the crate', async function () {
  let crate = db.crate.create({ name: 'test-crate-not-owner' });
  db.version.create({ crate });

  let owner = db.user.create();
  db.crateOwnership.create({
    crate,
    user: owner,
  });

  // Create GitLab config
  let config = db.trustpubGitlabConfig.create({
    crate,
    namespace: 'rust-lang',
    project: 'crates.io',
    workflow_filepath: '.gitlab-ci.yml',
    created_at: '2023-01-01T00:00:00Z',
  });

  // Login as a different user
  let user = db.user.create();
  db.mswSession.create({ user });

  let response = await fetch(`/api/v1/trusted_publishing/gitlab_configs/${config.id}`, {
    method: 'DELETE',
  });

  assert.strictEqual(response.status, 400);
  assert.deepEqual(await response.json(), {
    errors: [{ detail: 'You are not an owner of this crate' }],
  });
});
