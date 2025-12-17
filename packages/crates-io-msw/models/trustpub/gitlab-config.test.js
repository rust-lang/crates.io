import { test } from 'vitest';

import { db } from '../../index.js';

test('defaults are applied', async ({ expect }) => {
  let crate = await db.crate.create({});
  let config = await db.trustpubGitlabConfig.create({ crate });
  expect(config).toMatchInlineSnapshot(`
    {
      "crate": {
        "_extra_downloads": [],
        "badges": [],
        "categories": [],
        "created_at": "2010-06-16T21:30:45Z",
        "description": "This is the description for the crate called "crate-1"",
        "documentation": null,
        "downloads": 37035,
        "homepage": null,
        "id": 1,
        "keywords": [],
        "name": "crate-1",
        "recent_downloads": 321,
        "repository": null,
        "trustpubOnly": false,
        "updated_at": "2017-02-24T12:34:56Z",
      },
      "created_at": "2023-01-01T00:00:00Z",
      "environment": null,
      "id": 1,
      "namespace": "rust-lang",
      "namespace_id": null,
      "project": "repo-1",
      "workflow_filepath": ".gitlab-ci.yml",
    }
  `);
});

test('fields can be set', async ({ expect }) => {
  let crate = await db.crate.create({ name: 'serde' });
  let config = await db.trustpubGitlabConfig.create({
    crate,
    namespace: 'serde-rs',
    namespace_id: '12345',
    project: 'serde',
    workflow_filepath: '.gitlab/ci.yml',
    environment: 'production',
  });
  expect(config).toMatchInlineSnapshot(`
    {
      "crate": {
        "_extra_downloads": [],
        "badges": [],
        "categories": [],
        "created_at": "2010-06-16T21:30:45Z",
        "description": "This is the description for the crate called "serde"",
        "documentation": null,
        "downloads": 37035,
        "homepage": null,
        "id": 1,
        "keywords": [],
        "name": "serde",
        "recent_downloads": 321,
        "repository": null,
        "trustpubOnly": false,
        "updated_at": "2017-02-24T12:34:56Z",
      },
      "created_at": "2023-01-01T00:00:00Z",
      "environment": "production",
      "id": 1,
      "namespace": "serde-rs",
      "namespace_id": "12345",
      "project": "serde",
      "workflow_filepath": ".gitlab/ci.yml",
    }
  `);
});
