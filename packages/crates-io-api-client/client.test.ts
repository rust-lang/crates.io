import { db, handlers } from '@crates-io/msw';
import { setupServer } from 'msw/node';
import { afterAll, afterEach, beforeAll, expect, test } from 'vitest';

import { createClient } from './index.js';

const baseUrl = 'https://crates.io/';
globalThis.location = { href: baseUrl } as Location;

const server = setupServer(...handlers);

beforeAll(() => server.listen());
afterEach(() => server.resetHandlers());
afterEach(() => db.reset());
afterAll(() => server.close());

test('GET /api/v1/site_metadata', async () => {
  let client = createClient({ baseUrl });
  let response = await client.GET('/api/v1/site_metadata');

  expect(response).toMatchInlineSnapshot(`
    {
      "data": {
        "commit": "5048d31943118c6d67359bd207d307c854e82f45",
        "deployed_sha": "5048d31943118c6d67359bd207d307c854e82f45",
        "read_only": false,
      },
      "response": HttpResponse {
        "url": "https://crates.io/api/v1/site_metadata",
        Symbol(bodyType): null,
      },
    }
  `);
});

test('GET /api/v1/crates/{name}', async () => {
  let crate = await db.crate.create({ name: 'serde' });
  await db.version.create({ crate, num: '1.0.0' });

  let client = createClient({ baseUrl });
  let response = await client.GET('/api/v1/crates/{name}', {
    params: {
      path: { name: 'serde' },
      query: { include: '' },
    },
  });

  expect(response.data.crate.name).toBe('serde');
  expect(response).toMatchInlineSnapshot(`
    {
      "data": {
        "categories": null,
        "crate": {
          "badges": [],
          "categories": null,
          "created_at": "2010-06-16T21:30:45Z",
          "default_version": "1.0.0",
          "description": "This is the description for the crate called "serde"",
          "documentation": null,
          "downloads": 37035,
          "homepage": null,
          "id": "serde",
          "keywords": null,
          "links": {
            "owner_team": "/api/v1/crates/serde/owner_team",
            "owner_user": "/api/v1/crates/serde/owner_user",
            "reverse_dependencies": "/api/v1/crates/serde/reverse_dependencies",
            "version_downloads": "/api/v1/crates/serde/downloads",
            "versions": "/api/v1/crates/serde/versions",
          },
          "max_stable_version": null,
          "max_version": "0.0.0",
          "name": "serde",
          "newest_version": "0.0.0",
          "num_versions": 1,
          "recent_downloads": 321,
          "repository": null,
          "trustpub_only": false,
          "updated_at": "2017-02-24T12:34:56Z",
          "versions": null,
          "yanked": false,
        },
        "keywords": null,
        "versions": null,
      },
      "response": HttpResponse {
        "url": "https://crates.io/api/v1/crates/serde?include=",
        Symbol(bodyType): null,
      },
    }
  `);
});

test('GET /api/v1/crates/{name} error', async () => {
  let client = createClient({ baseUrl });
  let response = await client.GET('/api/v1/crates/{name}', {
    params: { path: { name: 'serde' } },
  });

  expect(response).toMatchInlineSnapshot(`
    {
      "error": {
        "errors": [
          {
            "detail": "Not Found",
          },
        ],
      },
      "response": HttpResponse {
        "url": "https://crates.io/api/v1/crates/serde",
        Symbol(bodyType): null,
      },
    }
  `);
});
