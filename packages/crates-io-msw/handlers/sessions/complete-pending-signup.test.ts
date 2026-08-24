import { expect, test } from 'vitest';

import { db } from '../../index.js';

test('creates a user and session from the pending signup', async function () {
  await db.pendingSignup.create({ login: 'ghost', email: 'ghost@example.com' });

  let response = await fetch('/api/private/session/signup', {
    method: 'POST',
    body: JSON.stringify({ signup: { email: 'new-user@example.com' } }),
  });

  expect(response.status).toBe(200);
  expect(await response.json()).toMatchInlineSnapshot(`
    {
      "owned_crates": [],
      "user": {
        "avatar": "https://avatars1.githubusercontent.com/u/14631425?v=4",
        "created_at": null,
        "email": "new-user@example.com",
        "email_verification_sent": true,
        "email_verified": false,
        "id": 1,
        "is_admin": false,
        "login": "ghost",
        "name": "User 1",
        "publish_notifications": true,
        "url": "https://github.com/ghost",
      },
    }
  `);

  expect(db.pendingSignup.findFirst()).toBeFalsy();
  expect(db.mswSession.findFirst()?.user.login).toBe('ghost');
});

test('returns an error without a pending signup', async function () {
  let response = await fetch('/api/private/session/signup', {
    method: 'POST',
    body: JSON.stringify({ signup: { email: 'new-user@example.com' } }),
  });

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

test('preserves the pending signup when the email is empty', async function () {
  await db.pendingSignup.create({ login: 'ghost' });

  let response = await fetch('/api/private/session/signup', {
    method: 'POST',
    body: JSON.stringify({ signup: { email: '' } }),
  });

  expect(response.status).toBe(422);
  expect(await response.json()).toMatchInlineSnapshot(`
    {
      "errors": [
        {
          "detail": "Please enter an email address.",
        },
      ],
    }
  `);

  expect(db.pendingSignup.findFirst()?.login).toBe('ghost');
  expect(db.mswSession.findFirst()).toBeFalsy();
});

test('preserves the pending signup when the email is invalid', async function () {
  await db.pendingSignup.create({ login: 'ghost' });

  let response = await fetch('/api/private/session/signup', {
    method: 'POST',
    body: JSON.stringify({ signup: { email: 'not-an-email' } }),
  });

  expect(response.status).toBe(422);
  expect(await response.json()).toMatchInlineSnapshot(`
    {
      "errors": [
        {
          "detail": "Please enter a valid email address.",
        },
      ],
    }
  `);

  expect(db.pendingSignup.findFirst()?.login).toBe('ghost');
  expect(db.mswSession.findFirst()).toBeFalsy();
});
