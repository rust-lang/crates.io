import { expect, test } from 'vitest';

import { db } from '../../index.js';

test('returns the first pending signup', async function () {
  await db.pendingSignup.create({ login: 'first', name: 'First User', email: null });
  await db.pendingSignup.create({ login: 'second', email: 'second@crates.io' });

  let response = await fetch('/api/private/session/signup');

  expect(response.status).toBe(200);
  expect(await response.json()).toMatchInlineSnapshot(`
    {
      "signup": {
        "email": null,
        "login": "first",
        "name": "First User",
      },
    }
  `);
});

test('returns an error without a pending signup', async function () {
  let response = await fetch('/api/private/session/signup');

  expect(response.status).toBe(400);
  expect(await response.json()).toMatchInlineSnapshot(`
    {
      "errors": [
        {
          "detail": "Your signup session is missing or has expired. Please authenticate with GitHub again.",
        },
      ],
    }
  `);
});

test('returns null for a missing name', async function () {
  await db.pendingSignup.create({ login: 'ghost' });

  let response = await fetch('/api/private/session/signup');

  expect(response.status).toBe(200);
  expect(await response.json()).toEqual({ signup: { login: 'ghost', name: null, email: null } });
});
