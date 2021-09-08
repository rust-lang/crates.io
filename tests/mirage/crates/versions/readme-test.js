import { module, test } from 'qunit';

import fetch from 'fetch';

import { setupTest } from '../../../helpers';
import setupMirage from '../../../helpers/setup-mirage';

module('Mirage | GET /api/v1/crates/:id/:version/readme', function (hooks) {
  setupTest(hooks);
  setupMirage(hooks);

  test('returns 404 for unknown crates', async function (assert) {
    let response = await fetch('/api/v1/crates/foo/1.0.0/readme');
    assert.equal(response.status, 404);
    assert.equal(await response.text(), '');
  });

  test('returns 404 for unknown versions', async function (assert) {
    this.server.create('crate', { name: 'rand' });

    let response = await fetch('/api/v1/crates/rand/1.0.0/readme');
    assert.equal(response.status, 404);
    assert.equal(await response.text(), '');
  });

  test('returns 404 for versions without README', async function (assert) {
    let crate = this.server.create('crate', { name: 'rand' });
    this.server.create('version', { crate, num: '1.0.0' });

    let response = await fetch('/api/v1/crates/rand/1.0.0/readme');
    assert.equal(response.status, 404);
    assert.equal(await response.text(), '');
  });

  test('returns the README as raw HTML', async function (assert) {
    let readme = 'lorem ipsum <i>est</i> dolor!';

    let crate = this.server.create('crate', { name: 'rand' });
    this.server.create('version', { crate, num: '1.0.0', readme: readme });

    let response = await fetch('/api/v1/crates/rand/1.0.0/readme');
    assert.equal(response.status, 200);
    assert.equal(await response.text(), readme);
  });
});
