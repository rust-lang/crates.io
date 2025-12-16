import { expect, test } from 'vitest';

import { db } from '../../index.js';

test('returns 404 for unknown crates', async function () {
  let response = await fetch('/api/v1/crates/foo/1.0.0/dependencies');
  expect(response.status).toBe(404);
  expect(await response.json()).toEqual({ errors: [{ detail: 'Not Found' }] });
});

test('returns 404 for unknown versions', async function () {
  await db.crate.create({ name: 'rand' });

  let response = await fetch('/api/v1/crates/rand/1.0.0/dependencies');
  expect(response.status).toBe(404);
  expect(await response.json()).toEqual({ errors: [{ detail: 'crate `rand` does not have a version `1.0.0`' }] });
});

test('empty case', async function () {
  let crate = await db.crate.create({ name: 'rand' });
  await db.version.create({ crate, num: '1.0.0' });

  let response = await fetch('/api/v1/crates/rand/1.0.0/dependencies');
  expect(response.status).toBe(200);
  expect(await response.json()).toEqual({
    dependencies: [],
  });
});

test('returns a list of dependencies belonging to the specified crate version', async function () {
  let crate = await db.crate.create({ name: 'rand' });
  let version = await db.version.create({ crate, num: '1.0.0' });

  let foo = await db.crate.create({ name: 'foo' });
  await db.dependency.create({ crate: foo, version });
  let bar = await db.crate.create({ name: 'bar' });
  await db.dependency.create({ crate: bar, version });
  let baz = await db.crate.create({ name: 'baz' });
  await db.dependency.create({ crate: baz, version });

  let response = await fetch('/api/v1/crates/rand/1.0.0/dependencies');
  expect(response.status).toBe(200);
  expect(await response.json()).toEqual({
    dependencies: [
      {
        id: 1,
        crate_id: 'foo',
        default_features: false,
        features: [],
        kind: 'normal',
        optional: true,
        req: '^2.1.3',
        target: null,
        version_id: 1,
      },
      {
        id: 2,
        crate_id: 'bar',
        default_features: false,
        features: [],
        kind: 'normal',
        optional: true,
        req: '0.3.7',
        target: null,
        version_id: 1,
      },
      {
        id: 3,
        crate_id: 'baz',
        default_features: true,
        features: [],
        kind: 'dev',
        optional: false,
        req: '~5.2.12',
        target: null,
        version_id: 1,
      },
    ],
  });
});
