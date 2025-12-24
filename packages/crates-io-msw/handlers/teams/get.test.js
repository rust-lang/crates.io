import { expect, test } from 'vitest';

import { db } from '../../index.js';

test('returns 404 for unknown teams', async function () {
  let response = await fetch('/api/v1/teams/foo');
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

test('returns a team object for known teams', async function () {
  let team = await db.team.create({ name: 'maintainers' });

  let response = await fetch(`/api/v1/teams/${team.login}`);
  expect(response.status).toBe(200);
  expect(await response.json()).toMatchInlineSnapshot(`
    {
      "team": {
        "avatar": "https://avatars1.githubusercontent.com/u/14631425?v=4",
        "id": 1,
        "login": "github:rust-lang:maintainers",
        "name": "maintainers",
        "url": "https://github.com/rust-lang",
      },
    }
  `);
});
