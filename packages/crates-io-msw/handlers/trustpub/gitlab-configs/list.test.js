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

  // Create GitLab configs
  let config1 = await db.trustpubGitlabConfig.create({
    crate,
    namespace: 'rust-lang',
    namespace_id: null,
    project: 'crates.io',
    workflow_filepath: '.gitlab-ci.yml',
    created_at: '2023-01-01T00:00:00Z',
  });

  let config2 = await db.trustpubGitlabConfig.create({
    crate,
    namespace: 'rust-lang',
    namespace_id: '12345',
    project: 'cargo',
    workflow_filepath: '.gitlab/ci.yml',
    environment: 'production',
    created_at: '2023-02-01T00:00:00Z',
  });

  let response = await fetch(`/api/v1/trusted_publishing/gitlab_configs?crate=${crate.name}`);
  expect(response.status).toBe(200);
  expect(await response.json()).toEqual({
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
  let crate = await db.crate.create({ name: 'test-crate-empty' });
  await db.version.create({ crate });

  let user = await db.user.create({ email_verified: true });
  await db.mswSession.create({ user });

  // Create crate ownership
  await db.crateOwnership.create({
    crate,
    user,
  });

  let response = await fetch(`/api/v1/trusted_publishing/gitlab_configs?crate=${crate.name}`);
  expect(response.status).toBe(200);
  expect(await response.json()).toEqual({
    gitlab_configs: [],
  });
});

test('returns 403 if unauthenticated', async function () {
  let response = await fetch(`/api/v1/trusted_publishing/gitlab_configs?crate=test-crate`);
  expect(response.status).toBe(403);
  expect(await response.json()).toEqual({
    errors: [{ detail: 'must be logged in to perform that action' }],
  });
});

test('returns 400 if query params are missing', async function () {
  let user = await db.user.create();
  await db.mswSession.create({ user });

  let response = await fetch(`/api/v1/trusted_publishing/gitlab_configs`);
  expect(response.status).toBe(400);
  expect(await response.json()).toEqual({
    errors: [{ detail: 'missing or invalid filter' }],
  });
});

test("returns 404 if crate can't be found", async function () {
  let user = await db.user.create();
  await db.mswSession.create({ user });

  let response = await fetch(`/api/v1/trusted_publishing/gitlab_configs?crate=nonexistent`);
  expect(response.status).toBe(404);
  expect(await response.json()).toEqual({
    errors: [{ detail: 'Not Found' }],
  });
});

test('returns 400 if user is not an owner of the crate', async function () {
  let crate = await db.crate.create({ name: 'test-crate-not-owner' });
  await db.version.create({ crate });

  let user = await db.user.create();
  await db.mswSession.create({ user });

  let response = await fetch(`/api/v1/trusted_publishing/gitlab_configs?crate=${crate.name}`);
  expect(response.status).toBe(400);
  expect(await response.json()).toEqual({
    errors: [{ detail: 'You are not an owner of this crate' }],
  });
});
