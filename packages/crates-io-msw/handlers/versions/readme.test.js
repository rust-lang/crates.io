import { expect, test } from 'vitest';

import { db } from '../../index.js';

test('returns 404 for unknown crates', async function () {
  let response = await fetch('/api/v1/crates/foo/1.0.0/readme');
  expect(response.status).toBe(403);
  expect(await response.text()).toBe('');
});

test('returns 404 for unknown versions', async function () {
  await db.crate.create({ name: 'rand' });

  let response = await fetch('/api/v1/crates/rand/1.0.0/readme');
  expect(response.status).toBe(403);
  expect(await response.text()).toBe('');
});

test('returns 404 for versions without README', async function () {
  let crate = await db.crate.create({ name: 'rand' });
  await db.version.create({ crate, num: '1.0.0' });

  let response = await fetch('/api/v1/crates/rand/1.0.0/readme');
  expect(response.status).toBe(403);
  expect(await response.text()).toBe('');
});

test('returns the README as raw HTML', async function () {
  let readme = 'lorem ipsum <i>est</i> dolor!';

  let crate = await db.crate.create({ name: 'rand' });
  await db.version.create({ crate, num: '1.0.0', readme: readme });

  let response = await fetch('/api/v1/crates/rand/1.0.0/readme');
  expect(response.status).toBe(200);
  expect(await response.text()).toBe(readme);
});
