import { module, test } from 'qunit';

import fetch from 'fetch';

import { setupTest } from '../../../helpers';
import setupMirage from '../../../helpers/setup-mirage';

const YANK_BODY = JSON.stringify({
  version: {
    yanked: true,
    yank_message: 'some reason',
  },
});

const UNYANK_BODY = JSON.stringify({
  version: {
    yanked: false,
  },
});

module('Mirage | PATCH /api/v1/crates/:crate/:version', function (hooks) {
  setupTest(hooks);
  setupMirage(hooks);

  test('returns 403 if unauthenticated', async function (assert) {
    let response = await fetch('/api/v1/crates/foo/1.0.0', { method: 'PATCH', body: YANK_BODY });
    assert.strictEqual(response.status, 403);
    assert.deepEqual(await response.json(), {
      errors: [{ detail: 'must be logged in to perform that action' }],
    });
  });

  test('returns 404 for unknown crates', async function (assert) {
    let user = this.server.create('user');
    this.authenticateAs(user);

    let response = await fetch('/api/v1/crates/foo/1.0.0', { method: 'PATCH', body: YANK_BODY });
    assert.strictEqual(response.status, 404);
    assert.deepEqual(await response.json(), { errors: [{ detail: 'Not Found' }] });
  });

  test('returns 404 for unknown versions', async function (assert) {
    this.server.create('crate', { name: 'foo' });

    let user = this.server.create('user');
    this.authenticateAs(user);

    let response = await fetch('/api/v1/crates/foo/1.0.0', { method: 'PATCH', body: YANK_BODY });
    assert.strictEqual(response.status, 404);
    assert.deepEqual(await response.json(), { errors: [{ detail: 'Not Found' }] });
  });

  test('yanks the version', async function (assert) {
    let crate = this.server.create('crate', { name: 'foo' });
    let version = this.server.create('version', { crate, num: '1.0.0', yanked: false });
    assert.false(version.yanked);
    assert.strictEqual(version.yank_message, null);

    let user = this.server.create('user');
    this.authenticateAs(user);

    let response = await fetch('/api/v1/crates/foo/1.0.0', { method: 'PATCH', body: YANK_BODY });
    assert.strictEqual(response.status, 200);
    assert.deepEqual(await response.json(), {
      version: {
        crate: 'foo',
        crate_size: 0,
        created_at: '2010-06-16T21:30:45Z',
        dl_path: '/api/v1/crates/foo/1.0.0/download',
        downloads: 0,
        id: '1',
        license: 'MIT/Apache-2.0',
        links: {
          dependencies: '/api/v1/crates/foo/1.0.0/dependencies',
          version_downloads: '/api/v1/crates/foo/1.0.0/downloads',
        },
        num: '1.0.0',
        published_by: null,
        readme_path: '/api/v1/crates/foo/1.0.0/readme',
        rust_version: null,
        updated_at: '2017-02-24T12:34:56Z',
        yank_message: 'some reason',
        yanked: true,
      },
    });

    user.reload();
    assert.true(version.yanked);
    assert.strictEqual(version.yank_message, 'some reason');

    response = await fetch('/api/v1/crates/foo/1.0.0', { method: 'PATCH', body: UNYANK_BODY });
    assert.strictEqual(response.status, 200);
    assert.deepEqual(await response.json(), {
      version: {
        crate: 'foo',
        crate_size: 0,
        created_at: '2010-06-16T21:30:45Z',
        dl_path: '/api/v1/crates/foo/1.0.0/download',
        downloads: 0,
        id: '1',
        license: 'MIT/Apache-2.0',
        links: {
          dependencies: '/api/v1/crates/foo/1.0.0/dependencies',
          version_downloads: '/api/v1/crates/foo/1.0.0/downloads',
        },
        num: '1.0.0',
        published_by: null,
        readme_path: '/api/v1/crates/foo/1.0.0/readme',
        rust_version: null,
        updated_at: '2017-02-24T12:34:56Z',
        yank_message: null,
        yanked: false,
      },
    });

    user.reload();
    assert.false(version.yanked);
    assert.strictEqual(version.yank_message, null);
  });
});
