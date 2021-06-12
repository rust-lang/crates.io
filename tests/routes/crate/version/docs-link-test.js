import { visit } from '@ember/test-helpers';
import { module, test } from 'qunit';

import { setupApplicationTest } from 'cargo/tests/helpers';

module('Route | crate.version | docs link', function (hooks) {
  setupApplicationTest(hooks);

  test('shows regular documentation link', async function (assert) {
    let crate = this.server.create('crate', { name: 'foo', documentation: 'https://foo.io/docs' });
    this.server.create('version', { crate, num: '1.0.0' });

    await visit('/crates/foo');
    assert.dom('[data-test-docs-link] a').hasAttribute('href', 'https://foo.io/docs');
  });

  test('show no docs link if `documentation` is unspecified and there are no related docs.rs builds', async function (assert) {
    let crate = this.server.create('crate', { name: 'foo' });
    this.server.create('version', { crate, num: '1.0.0' });

    this.server.get('https://docs.rs/crate/:crate/:version/builds.json', []);

    await visit('/crates/foo');
    assert.dom('[data-test-docs-link] a').doesNotExist();
  });

  test('show docs link if `documentation` is unspecified and there are related docs.rs builds', async function (assert) {
    let crate = this.server.create('crate', { name: 'foo' });
    this.server.create('version', { crate, num: '1.0.0' });

    this.server.get('https://docs.rs/crate/:crate/:version/builds.json', [
      {
        id: 42,
        rustc_version: 'rustc 1.50.0-nightly (1c389ffef 2020-11-24)',
        docsrs_version: 'docsrs 0.6.0 (31c864e 2020-11-22)',
        build_status: true,
        build_time: '2020-12-06T09:04:36.610302Z',
        output: null,
      },
    ]);

    await visit('/crates/foo');
    assert.dom('[data-test-docs-link] a').hasAttribute('href', 'https://docs.rs/foo/1.0.0');
  });

  test('show original docs link if `documentation` points to docs.rs and there are no related docs.rs builds', async function (assert) {
    let crate = this.server.create('crate', { name: 'foo', documentation: 'https://docs.rs/foo/0.6.2' });
    this.server.create('version', { crate, num: '1.0.0' });

    this.server.get('https://docs.rs/crate/:crate/:version/builds.json', []);

    await visit('/crates/foo');
    assert.dom('[data-test-docs-link] a').hasAttribute('href', 'https://docs.rs/foo/0.6.2');
  });

  test('show updated docs link if `documentation` points to docs.rs and there are related docs.rs builds', async function (assert) {
    let crate = this.server.create('crate', { name: 'foo', documentation: 'https://docs.rs/foo/0.6.2' });
    this.server.create('version', { crate, num: '1.0.0' });

    this.server.get('https://docs.rs/crate/:crate/:version/builds.json', [
      {
        id: 42,
        rustc_version: 'rustc 1.50.0-nightly (1c389ffef 2020-11-24)',
        docsrs_version: 'docsrs 0.6.0 (31c864e 2020-11-22)',
        build_status: true,
        build_time: '2020-12-06T09:04:36.610302Z',
        output: null,
      },
    ]);

    await visit('/crates/foo');
    assert.dom('[data-test-docs-link] a').hasAttribute('href', 'https://docs.rs/foo/1.0.0');
  });

  test('ajax errors are ignored', async function (assert) {
    let crate = this.server.create('crate', { name: 'foo', documentation: 'https://docs.rs/foo/0.6.2' });
    this.server.create('version', { crate, num: '1.0.0' });

    this.server.get('https://docs.rs/crate/:crate/:version/builds.json', {}, 500);

    await visit('/crates/foo');
    assert.dom('[data-test-docs-link] a').hasAttribute('href', 'https://docs.rs/foo/0.6.2');
  });

  test('null builds in docs.rs responses are ignored', async function (assert) {
    let crate = this.server.create('crate', { name: 'foo', documentation: 'https://docs.rs/foo/0.6.2' });
    this.server.create('version', { crate, num: '0.6.2' });

    this.server.get('https://docs.rs/crate/:crate/:version/builds.json', [null]);

    await visit('/crates/foo');
    assert.dom('[data-test-docs-link] a').hasAttribute('href', 'https://docs.rs/foo/0.6.2');
  });

  test('empty arrays in docs.rs responses are ignored', async function (assert) {
    let crate = this.server.create('crate', { name: 'foo', documentation: 'https://docs.rs/foo/0.6.2' });
    this.server.create('version', { crate, num: '0.6.2' });

    this.server.get('https://docs.rs/crate/:crate/:version/builds.json', []);

    await visit('/crates/foo');
    assert.dom('[data-test-docs-link] a').hasAttribute('href', 'https://docs.rs/foo/0.6.2');
  });
});
