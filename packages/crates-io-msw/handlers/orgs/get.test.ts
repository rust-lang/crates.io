import { expect, test } from 'vitest';

import { db } from '../../index.js';

test('returns empty teams for unknown orgs', async function () {
  let response = await fetch('/api/v1/orgs/foo');
  expect(response.status).toBe(200);
  expect(await response.json()).toMatchInlineSnapshot(`
    {
      "teams": [],
    }
  `);
});

test('returns a teams list for known orgs', async function () {
  await db.team.create({ org: 'rust-lang', name: 'maintainers' });
  await db.team.create({ org: 'rust-lang', name: 'readers' });
  await db.team.create({ org: 'foo', name: 'maintainers' });

  let response = await fetch(`/api/v1/orgs/github:rust-lang`);
  expect(response.status).toBe(200);
  expect(await response.json()).toMatchInlineSnapshot(`
    {
      "teams": [
        {
          "avatar": "https://avatars1.githubusercontent.com/u/14631425?v=4",
          "id": 1,
          "login": "github:rust-lang:maintainers",
          "name": "maintainers",
          "url": "https://github.com/rust-lang",
        },
        {
          "avatar": "https://avatars1.githubusercontent.com/u/14631425?v=4",
          "id": 2,
          "login": "github:rust-lang:readers",
          "name": "readers",
          "url": "https://github.com/rust-lang",
        },
      ],
    }
  `);
});
