import { expect, test } from 'vitest';

import { db } from '../../index.js';

test('returns 404 for unknown crates', async function () {
  let response = await fetch('/api/v1/crates/foo/1.0.0/downloads');
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

test('returns 404 for unknown versions', async function () {
  await db.crate.create({ name: 'rand' });

  let response = await fetch('/api/v1/crates/rand/1.0.0/downloads');
  expect(response.status).toBe(404);
  expect(await response.json()).toMatchInlineSnapshot(`
    {
      "errors": [
        {
          "detail": "crate \`rand\` does not have a version \`1.0.0\`",
        },
      ],
    }
  `);
});

test('empty case', async function () {
  let crate = await db.crate.create({ name: 'rand' });
  await db.version.create({ crate, num: '1.0.0' });

  let response = await fetch('/api/v1/crates/rand/1.0.0/downloads');
  expect(response.status).toBe(200);
  expect(await response.json()).toMatchInlineSnapshot(`
    {
      "version_downloads": [],
    }
  `);
});

test('returns a list of version downloads belonging to the specified crate version', async function () {
  let crate = await db.crate.create({ name: 'rand' });
  let version = await db.version.create({ crate, num: '1.0.0' });
  await db.versionDownload.create({ version, date: '2020-01-13' });
  await db.versionDownload.create({ version, date: '2020-01-14' });
  await db.versionDownload.create({ version, date: '2020-01-15' });

  let response = await fetch('/api/v1/crates/rand/1.0.0/downloads');
  expect(response.status).toBe(200);
  expect(await response.json()).toMatchInlineSnapshot(`
    {
      "version_downloads": [
        {
          "date": "2020-01-13",
          "downloads": 7035,
          "version": 1,
        },
        {
          "date": "2020-01-14",
          "downloads": 14070,
          "version": 1,
        },
        {
          "date": "2020-01-15",
          "downloads": 21105,
          "version": 1,
        },
      ],
    }
  `);
});
