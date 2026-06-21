import { expect, test } from 'vitest';

import { db } from '../index.js';

test('returns 403 for unknown crates', async function () {
  let response = await fetch('https://static.crates.io/readmes/foo/foo-1.0.0.html');
  expect(response.status).toBe(403);
  expect(await response.text()).toBe('');
});

test('returns 403 for unknown versions', async function () {
  await db.crate.create({ name: 'rand' });

  let response = await fetch('https://static.crates.io/readmes/rand/rand-1.0.0.html');
  expect(response.status).toBe(403);
  expect(await response.text()).toBe('');
});

test('returns 403 for versions without README', async function () {
  let crate = await db.crate.create({ name: 'rand' });
  await db.version.create({ crate, num: '1.0.0' });

  let response = await fetch('https://static.crates.io/readmes/rand/rand-1.0.0.html');
  expect(response.status).toBe(403);
  expect(await response.text()).toBe('');
});

test('returns the README as raw HTML', async function () {
  let readme = 'lorem ipsum <i>est</i> dolor!';

  let crate = await db.crate.create({ name: 'rand' });
  await db.version.create({ crate, num: '1.0.0', readme });

  let response = await fetch('https://static.crates.io/readmes/rand/rand-1.0.0.html');
  expect(response.status).toBe(200);
  expect(await response.text()).toBe(readme);
});

test('recovers the version for crate names containing dashes', async function () {
  let readme = 'serde readme';

  let crate = await db.crate.create({ name: 'serde-json' });
  await db.version.create({ crate, num: '1.0.0', readme });

  let response = await fetch('https://static.crates.io/readmes/serde-json/serde-json-1.0.0.html');
  expect(response.status).toBe(200);
  expect(await response.text()).toBe(readme);
});

test('decodes the version from the encoded filename', async function () {
  let readme = 'build metadata readme';

  let crate = await db.crate.create({ name: 'rand' });
  await db.version.create({ crate, num: '1.0.0+foo', readme });

  let response = await fetch('https://static.crates.io/readmes/rand/rand-1.0.0%2Bfoo.html');
  expect(response.status).toBe(200);
  expect(await response.text()).toBe(readme);
});
