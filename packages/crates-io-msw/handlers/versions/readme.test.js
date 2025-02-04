import { assert, test } from 'vitest';

import { db } from '../../index.js';

test('returns 404 for unknown crates', async function () {
  let response = await fetch('/api/v1/crates/foo/1.0.0/readme');
  assert.strictEqual(response.status, 403);
  assert.strictEqual(await response.text(), '');
});

test('returns 404 for unknown versions', async function () {
  db.crate.create({ name: 'rand' });

  let response = await fetch('/api/v1/crates/rand/1.0.0/readme');
  assert.strictEqual(response.status, 403);
  assert.strictEqual(await response.text(), '');
});

test('returns 404 for versions without README', async function () {
  let crate = db.crate.create({ name: 'rand' });
  db.version.create({ crate, num: '1.0.0' });

  let response = await fetch('/api/v1/crates/rand/1.0.0/readme');
  assert.strictEqual(response.status, 403);
  assert.strictEqual(await response.text(), '');
});

test('returns the README as raw HTML', async function () {
  let readme = 'lorem ipsum <i>est</i> dolor!';

  let crate = db.crate.create({ name: 'rand' });
  db.version.create({ crate, num: '1.0.0', readme: readme });

  let response = await fetch('/api/v1/crates/rand/1.0.0/readme');
  assert.strictEqual(response.status, 200);
  assert.strictEqual(await response.text(), readme);
});
