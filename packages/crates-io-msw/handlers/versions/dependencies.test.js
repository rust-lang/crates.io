import { assert, test } from 'vitest';

import { db } from '../../index.js';

test('returns 404 for unknown crates', async function () {
  let response = await fetch('/api/v1/crates/foo/1.0.0/dependencies');
  assert.strictEqual(response.status, 404);
  assert.deepEqual(await response.json(), { errors: [{ detail: 'Not Found' }] });
});

test('returns 404 for unknown versions', async function () {
  db.crate.create({ name: 'rand' });

  let response = await fetch('/api/v1/crates/rand/1.0.0/dependencies');
  assert.strictEqual(response.status, 404);
  assert.deepEqual(await response.json(), { errors: [{ detail: 'crate `rand` does not have a version `1.0.0`' }] });
});

test('empty case', async function () {
  let crate = db.crate.create({ name: 'rand' });
  db.version.create({ crate, num: '1.0.0' });

  let response = await fetch('/api/v1/crates/rand/1.0.0/dependencies');
  assert.strictEqual(response.status, 200);
  assert.deepEqual(await response.json(), {
    dependencies: [],
  });
});

test('returns a list of dependencies belonging to the specified crate version', async function () {
  let crate = db.crate.create({ name: 'rand' });
  let version = db.version.create({ crate, num: '1.0.0' });

  let foo = db.crate.create({ name: 'foo' });
  db.dependency.create({ crate: foo, version });
  let bar = db.crate.create({ name: 'bar' });
  db.dependency.create({ crate: bar, version });
  let baz = db.crate.create({ name: 'baz' });
  db.dependency.create({ crate: baz, version });

  let response = await fetch('/api/v1/crates/rand/1.0.0/dependencies');
  assert.strictEqual(response.status, 200);
  assert.deepEqual(await response.json(), {
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
