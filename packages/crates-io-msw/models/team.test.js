import { test } from 'vitest';

import { db } from '../index.js';

test('default are applied', ({ expect }) => {
  let team = db.team.create();
  expect(team).toMatchInlineSnapshot(`
    {
      "avatar": "https://avatars1.githubusercontent.com/u/14631425?v=4",
      "id": 1,
      "login": "github:rust-lang:team-1",
      "name": "team-1",
      "org": "rust-lang",
      "url": "https://github.com/rust-lang",
      Symbol(type): "team",
      Symbol(primaryKey): "id",
    }
  `);
});

test('attributes can be set', ({ expect }) => {
  let team = db.team.create({ name: 'axum', org: 'tokio-rs' });
  expect(team).toMatchInlineSnapshot(`
    {
      "avatar": "https://avatars1.githubusercontent.com/u/14631425?v=4",
      "id": 1,
      "login": "github:tokio-rs:axum",
      "name": "axum",
      "org": "tokio-rs",
      "url": "https://github.com/tokio-rs",
      Symbol(type): "team",
      Symbol(primaryKey): "id",
    }
  `);
});
