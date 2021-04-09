import { render, settled, waitFor } from '@ember/test-helpers';
import { module, test } from 'qunit';

import { defer } from 'rsvp';

import { hbs } from 'ember-cli-htmlbars';

import { setupRenderingTest } from 'cargo/tests/helpers';

import setupMirage from '../../helpers/setup-mirage';

module('Component | CrateSidebar | Playground Button', function (hooks) {
  setupRenderingTest(hooks);
  setupMirage(hooks);

  hooks.beforeEach(function () {
    let crates = [
      { name: 'addr2line', version: '0.14.1', id: 'addr2line' },
      { name: 'adler', version: '0.2.3', id: 'adler' },
      { name: 'adler32', version: '1.2.0', id: 'adler32' },
      { name: 'ahash', version: '0.4.7', id: 'ahash' },
      { name: 'aho-corasick', version: '0.7.15', id: 'aho_corasick' },
      { name: 'ansi_term', version: '0.12.1', id: 'ansi_term' },
      { name: 'ansi_term', version: '0.11.0', id: 'ansi_term_0_11_0' },
    ];

    this.server.get('https://play.rust-lang.org/meta/crates', { crates });
  });

  test('button is hidden for unavailable crates', async function (assert) {
    let crate = this.server.create('crate', { name: 'foo' });
    this.server.create('version', { crate, num: '1.0.0' });

    let store = this.owner.lookup('service:store');
    this.crate = await store.findRecord('crate', crate.name);
    this.version = (await this.crate.versions).firstObject;

    await render(hbs`<CrateSidebar @crate={{this.crate}} @version={{this.version}} />`);
    assert.dom('[data-test-playground-button]').doesNotExist();
  });

  test('button is visible for available crates', async function (assert) {
    let crate = this.server.create('crate', { name: 'aho-corasick' });
    this.server.create('version', { crate, num: '1.0.0' });

    let store = this.owner.lookup('service:store');
    this.crate = await store.findRecord('crate', crate.name);
    this.version = (await this.crate.versions).firstObject;

    let expectedHref =
      'https://play.rust-lang.org/?edition=2018&code=use%20aho_corasick%3B%0A%0Afn%20main()%20%7B%0A%20%20%20%20%2F%2F%20try%20using%20the%20%60aho_corasick%60%20crate%20here%0A%7D';

    await render(hbs`<CrateSidebar @crate={{this.crate}} @version={{this.version}} />`);
    assert.dom('[data-test-playground-button]').hasAttribute('href', expectedHref);
  });

  test('button is hidden while Playground request is pending', async function (assert) {
    let crate = this.server.create('crate', { name: 'aho-corasick' });
    this.server.create('version', { crate, num: '1.0.0' });

    let deferred = defer();
    this.server.get('https://play.rust-lang.org/meta/crates', deferred.promise);

    let store = this.owner.lookup('service:store');
    this.crate = await store.findRecord('crate', crate.name);
    this.version = (await this.crate.versions).firstObject;

    render(hbs`<CrateSidebar @crate={{this.crate}} @version={{this.version}} />`);
    await waitFor('[data-test-owners]');
    assert.dom('[data-test-playground-button]').doesNotExist();

    deferred.resolve({ crates: [] });
    await settled();
  });

  test('button is hidden if the Playground request fails', async function (assert) {
    let crate = this.server.create('crate', { name: 'aho-corasick' });
    this.server.create('version', { crate, num: '1.0.0' });

    this.server.get('https://play.rust-lang.org/meta/crates', {}, 500);

    let store = this.owner.lookup('service:store');
    this.crate = await store.findRecord('crate', crate.name);
    this.version = (await this.crate.versions).firstObject;

    await render(hbs`<CrateSidebar @crate={{this.crate}} @version={{this.version}} />`);
    assert.dom('[data-test-playground-button]').doesNotExist();
  });
});
