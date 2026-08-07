import { test } from 'vitest';

import { db } from '../index.js';

test('throws if `login` is not set', async ({ expect }) => {
  // @ts-expect-error: missing required field
  await expect(() => db.pendingSignup.create({})).rejects.toThrowErrorMatchingInlineSnapshot(
    `[Error: Failed to create a new record with initial values: does not match the schema. Please see the schema validation errors above.]`,
  );
});

test('happy path', async ({ expect }) => {
  let pendingSignup = await db.pendingSignup.create({ login: 'ghost' });

  expect(pendingSignup).toMatchInlineSnapshot(`
    {
      "email": null,
      "id": 1,
      "login": "ghost",
    }
  `);
});
