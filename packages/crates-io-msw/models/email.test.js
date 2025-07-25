import { test } from 'vitest';

import { db } from '../index.js';

test('default are applied', ({ expect }) => {
  let email = db.email.create();
  expect(email).toMatchInlineSnapshot(`
    {
      "email": "foo@crates.io",
      "id": 1,
      "primary": false,
      "token": null,
      "verification_email_sent": false,
      "verified": false,
      Symbol(type): "email",
      Symbol(primaryKey): "id",
    }
  `);
});
