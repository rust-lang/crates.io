import { visit } from '@ember/test-helpers';
import { module, test } from 'qunit';

import { http, HttpResponse } from 'msw';

import { setupApplicationTest } from 'crates-io/tests/helpers';

module('Route | crate.version | source link', function (hooks) {
  setupApplicationTest(hooks);

  test('shows docs.rs source link even if non-docs.rs documentation link is specified', async function (assert) {
    let crate = await this.db.crate.create({ name: 'foo', documentation: 'https://foo.io/docs' });
    await this.db.version.create({ crate, num: '1.0.0' });

    let response = HttpResponse.json({ doc_status: false, version: '1.0.0' });
    this.worker.use(http.get('https://docs.rs/crate/:crate/:version/status.json', () => response));

    await visit('/crates/foo');
    assert.dom('[data-test-source-link] a').hasAttribute('href', 'https://docs.rs/crate/foo/1.0.0/source/');
  });

  test('show no source link if there are no related docs.rs builds', async function (assert) {
    let crate = await this.db.crate.create({ name: 'foo' });
    await this.db.version.create({ crate, num: '1.0.0' });

    let error = HttpResponse.text('not found', { status: 404 });
    this.worker.use(http.get('https://docs.rs/crate/:crate/:version/status.json', () => error));

    await visit('/crates/foo');
    assert.dom('[data-test-source-link] a').doesNotExist();
  });

  test('show source link if `documentation` is unspecified and there are related docs.rs builds', async function (assert) {
    let crate = await this.db.crate.create({ name: 'foo' });
    await this.db.version.create({ crate, num: '1.0.0' });

    let response = HttpResponse.json({ doc_status: true, version: '1.0.0' });
    this.worker.use(http.get('https://docs.rs/crate/:crate/:version/status.json', () => response));

    await visit('/crates/foo');
    assert.dom('[data-test-source-link] a').hasAttribute('href', 'https://docs.rs/crate/foo/1.0.0/source/');
  });

  test('show no source link if `documentation` points to docs.rs and there are no related docs.rs builds', async function (assert) {
    let crate = await this.db.crate.create({ name: 'foo', documentation: 'https://docs.rs/foo/0.6.2' });
    await this.db.version.create({ crate, num: '1.0.0' });

    let error = HttpResponse.text('not found', { status: 404 });
    this.worker.use(http.get('https://docs.rs/crate/:crate/:version/status.json', () => error));

    await visit('/crates/foo');
    assert.dom('[data-test-source-link] a').doesNotExist();
  });

  test('show source link if `documentation` points to docs.rs and there is a successful docs.rs response', async function (assert) {
    let crate = await this.db.crate.create({ name: 'foo', documentation: 'https://docs.rs/foo/0.6.2' });
    await this.db.version.create({ crate, num: '1.0.0' });

    let response = HttpResponse.json({ doc_status: false, version: '1.0.0' });
    this.worker.use(http.get('https://docs.rs/crate/:crate/:version/status.json', () => response));

    await visit('/crates/foo');
    assert.dom('[data-test-source-link] a').hasAttribute('href', 'https://docs.rs/crate/foo/1.0.0/source/');
  });

  test('ajax errors are ignored, but show no source link', async function (assert) {
    let crate = await this.db.crate.create({ name: 'foo', documentation: 'https://docs.rs/foo/0.6.2' });
    await this.db.version.create({ crate, num: '1.0.0' });

    let error = HttpResponse.text('error', { status: 500 });
    this.worker.use(http.get('https://docs.rs/crate/:crate/:version/status.json', () => error));

    await visit('/crates/foo');
    assert.dom('[data-test-source-link] a').doesNotExist();
  });

  test('empty docs.rs responses are ignored, still show source link', async function (assert) {
    let crate = await this.db.crate.create({ name: 'foo', documentation: 'https://docs.rs/foo/0.6.2' });
    await this.db.version.create({ crate, num: '0.6.2' });

    let response = HttpResponse.json({});
    this.worker.use(http.get('https://docs.rs/crate/:crate/:version/status.json', () => response));

    await visit('/crates/foo');
    assert.dom('[data-test-source-link] a').hasAttribute('href', 'https://docs.rs/crate/foo/0.6.2/source/');
  });
});
