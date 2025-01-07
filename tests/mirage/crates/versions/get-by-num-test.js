import { module, test } from 'qunit';

import fetch from 'fetch';

import { setupTest } from '../../../helpers';
import setupMirage from '../../../helpers/setup-mirage';

module('Mirage | GET /api/v1/crates/:name/:version', function (hooks) {
  setupTest(hooks);
  setupMirage(hooks);

  test('returns 404 for unknown crates', async function (assert) {
    let response = await fetch('/api/v1/crates/foo/1.0.0-beta.1');
    assert.strictEqual(response.status, 404);
    assert.deepEqual(await response.json(), { errors: [{ detail: 'Not Found' }] });
  });

  test('returns 404 for unknown versions', async function (assert) {
    let crate = this.server.create('crate', { name: 'rand' });
    this.server.create('version', { crate, num: '1.0.0-alpha.1' });
    let response = await fetch('/api/v1/crates/rand/1.0.0-beta.1');
    assert.strictEqual(response.status, 404);
    assert.deepEqual(await response.json(), {
      errors: [{ detail: 'crate `rand` does not have a version `1.0.0-beta.1`' }],
    });
  });

  test('returns a version object for known version', async function (assert) {
    let crate = this.server.create('crate', { name: 'rand' });
    this.server.create('version', { crate, num: '1.0.0-beta.1' });

    let response = await fetch('/api/v1/crates/rand/1.0.0-beta.1');
    assert.strictEqual(response.status, 200);
    assert.deepEqual(await response.json(), {
      version: {
        crate: 'rand',
        crate_size: 0,
        created_at: '2010-06-16T21:30:45Z',
        dl_path: '/api/v1/crates/rand/1.0.0-beta.1/download',
        downloads: 0,
        id: '1',
        license: 'MIT/Apache-2.0',
        links: {
          dependencies: '/api/v1/crates/rand/1.0.0-beta.1/dependencies',
          version_downloads: '/api/v1/crates/rand/1.0.0-beta.1/downloads',
        },
        num: '1.0.0-beta.1',
        published_by: null,
        readme_path: '/api/v1/crates/rand/1.0.0-beta.1/readme',
        rust_version: null,
        updated_at: '2017-02-24T12:34:56Z',
        yank_message: null,
        yanked: false,
      },
    });
  });
});
