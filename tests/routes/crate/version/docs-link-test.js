import { visit } from '@ember/test-helpers';
import { module, test } from 'qunit';

import { http, HttpResponse } from 'msw';

import { setupApplicationTest } from 'crates-io/tests/helpers';

module('Route | crate.version | docs link', function (hooks) {
  setupApplicationTest(hooks, { msw: true });

  test('shows regular documentation link', async function (assert) {
    let crate = this.db.crate.create({ name: 'foo', documentation: 'https://foo.io/docs' });
    this.db.version.create({ crate, num: '1.0.0' });

    await visit('/crates/foo');
    assert.dom('[data-test-docs-link] a').hasAttribute('href', 'https://foo.io/docs');
  });

  test('show no docs link if `documentation` is unspecified and there are no related docs.rs builds', async function (assert) {
    let crate = this.db.crate.create({ name: 'foo' });
    this.db.version.create({ crate, num: '1.0.0' });

    let error = HttpResponse.text('not found', { status: 404 });
    this.worker.use(http.get('https://docs.rs/crate/:crate/:version/status.json', () => error));

    await visit('/crates/foo');
    assert.dom('[data-test-docs-link] a').doesNotExist();
  });

  test('show docs link if `documentation` is unspecified and there are related docs.rs builds', async function (assert) {
    let crate = this.db.crate.create({ name: 'foo' });
    this.db.version.create({ crate, num: '1.0.0' });

    let response = HttpResponse.json({ doc_status: true, version: '1.0.0' });
    this.worker.use(http.get('https://docs.rs/crate/:crate/:version/status.json', () => response));

    await visit('/crates/foo');
    assert.dom('[data-test-docs-link] a').hasAttribute('href', 'https://docs.rs/foo/1.0.0');
  });

  test('show original docs link if `documentation` points to docs.rs and there are no related docs.rs builds', async function (assert) {
    let crate = this.db.crate.create({ name: 'foo', documentation: 'https://docs.rs/foo/0.6.2' });
    this.db.version.create({ crate, num: '1.0.0' });

    let error = HttpResponse.text('not found', { status: 404 });
    this.worker.use(http.get('https://docs.rs/crate/:crate/:version/status.json', () => error));

    await visit('/crates/foo');
    assert.dom('[data-test-docs-link] a').hasAttribute('href', 'https://docs.rs/foo/0.6.2');
  });

  test('show updated docs link if `documentation` points to docs.rs and there are related docs.rs builds', async function (assert) {
    let crate = this.db.crate.create({ name: 'foo', documentation: 'https://docs.rs/foo/0.6.2' });
    this.db.version.create({ crate, num: '1.0.0' });

    let response = HttpResponse.json({ doc_status: true, version: '1.0.0' });
    this.worker.use(http.get('https://docs.rs/crate/:crate/:version/status.json', () => response));

    await visit('/crates/foo');
    assert.dom('[data-test-docs-link] a').hasAttribute('href', 'https://docs.rs/foo/1.0.0');
  });

  test('ajax errors are ignored', async function (assert) {
    let crate = this.db.crate.create({ name: 'foo', documentation: 'https://docs.rs/foo/0.6.2' });
    this.db.version.create({ crate, num: '1.0.0' });

    let error = HttpResponse.text('error', { status: 500 });
    this.worker.use(http.get('https://docs.rs/crate/:crate/:version/status.json', () => error));

    await visit('/crates/foo');
    assert.dom('[data-test-docs-link] a').hasAttribute('href', 'https://docs.rs/foo/0.6.2');
  });

  test('empty docs.rs responses are ignored', async function (assert) {
    let crate = this.db.crate.create({ name: 'foo', documentation: 'https://docs.rs/foo/0.6.2' });
    this.db.version.create({ crate, num: '0.6.2' });

    let response = HttpResponse.json({});
    this.worker.use(http.get('https://docs.rs/crate/:crate/:version/status.json', () => response));

    await visit('/crates/foo');
    assert.dom('[data-test-docs-link] a').hasAttribute('href', 'https://docs.rs/foo/0.6.2');
  });
});
