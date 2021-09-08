import { module, test } from 'qunit';

import fetch from 'fetch';

import { setupTest } from '../../../helpers';
import setupMirage from '../../../helpers/setup-mirage';

module('Mirage | GET /api/v1/me/updates', function (hooks) {
  setupTest(hooks);
  setupMirage(hooks);

  test('returns 403 for unauthenticated user', async function (assert) {
    let response = await fetch('/api/v1/me/updates');
    assert.equal(response.status, 403);
    assert.deepEqual(await response.json(), {
      errors: [{ detail: 'must be logged in to perform that action' }],
    });
  });

  test('returns latest versions of followed crates', async function (assert) {
    let foo = this.server.create('crate', { name: 'foo' });
    this.server.create('version', { crate: foo, num: '1.2.3' });

    let bar = this.server.create('crate', { name: 'bar' });
    this.server.create('version', { crate: bar, num: '0.8.6' });

    let user = this.server.create('user', { followedCrates: [foo] });
    this.authenticateAs(user);

    let response = await fetch('/api/v1/me/updates');
    assert.equal(response.status, 200);
    assert.deepEqual(await response.json(), {
      versions: [
        {
          id: '1',
          crate: 'foo',
          crate_size: 0,
          created_at: '2010-06-16T21:30:45Z',
          dl_path: '/api/v1/crates/foo/1.2.3/download',
          downloads: 0,
          license: 'MIT/Apache-2.0',
          links: {
            dependencies: '/api/v1/crates/foo/1.2.3/dependencies',
            version_downloads: '/api/v1/crates/foo/1.2.3/downloads',
          },
          num: '1.2.3',
          published_by: null,
          readme_path: '/api/v1/crates/foo/1.2.3/readme',
          updated_at: '2017-02-24T12:34:56Z',
          yanked: false,
        },
      ],
      meta: {
        more: false,
      },
    });
  });

  test('empty case', async function (assert) {
    let user = this.server.create('user');
    this.authenticateAs(user);

    let response = await fetch('/api/v1/me/updates');
    assert.equal(response.status, 200);
    assert.deepEqual(await response.json(), {
      versions: [],
      meta: { more: false },
    });
  });

  test('supports pagination', async function (assert) {
    let crate = this.server.create('crate', { name: 'foo' });
    this.server.createList('version', 25, { crate });

    let user = this.server.create('user', { followedCrates: [crate] });
    this.authenticateAs(user);

    let response = await fetch('/api/v1/me/updates?page=2');
    assert.equal(response.status, 200);

    let responsePayload = await response.json();
    assert.equal(responsePayload.versions.length, 10);
    assert.deepEqual(
      responsePayload.versions.map(it => it.id),
      ['15', '14', '13', '12', '11', '10', '9', '8', '7', '6'],
    );
    assert.deepEqual(responsePayload.meta, { more: true });
  });
});
