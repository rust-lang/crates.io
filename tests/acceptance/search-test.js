import { blur, click, currentURL, fillIn, settled, triggerEvent, visit, waitFor } from '@ember/test-helpers';
import { module, test } from 'qunit';

import { defer } from 'rsvp';

import percySnapshot from '@percy/ember';
import a11yAudit from 'ember-a11y-testing/test-support/audit';
import { keyDown } from 'ember-keyboard/test-support/test-helpers';
import { getPageTitle } from 'ember-page-title/test-support';

import { setupApplicationTest } from 'cargo/tests/helpers';

import { list as listCrates } from '../../mirage/route-handlers/crates';
import axeConfig from '../axe-config';

module('Acceptance | search', function (hooks) {
  setupApplicationTest(hooks);

  test('searching for "rust"', async function (assert) {
    this.server.loadFixtures();

    await visit('/');
    await fillIn('[data-test-search-input]', 'rust');
    await triggerEvent('[data-test-search-form]', 'submit');

    assert.equal(currentURL(), '/search?q=rust');
    assert.equal(getPageTitle(), "Search Results for 'rust' - crates.io: Rust Package Registry");

    assert.dom('[data-test-header]').hasText("Search Results for 'rust'");
    assert.dom('[data-test-search-nav]').hasText('Displaying 1-7 of 7 total results');
    assert
      .dom('[data-test-search-sort]')
      .hasText('Sort by Relevance Relevance All-Time Downloads Recent Downloads Recent Updates Newly Added');
    assert.dom('[data-test-crate-row="0"] [data-test-crate-link]').hasText('kinetic-rust');
    assert.dom('[data-test-crate-row="0"] [data-test-version]').hasText('v0.0.16');

    assert
      .dom('[data-test-crate-row="0"] [data-test-description]')
      .hasText('A Kinetic protocol library written in Rust');
    assert.dom('[data-test-crate-row="0"] [data-test-downloads]').hasText('All-Time: 225');
    assert.dom('[data-test-crate-row="0"] [data-test-updated-at]').exists();

    await percySnapshot(assert);
    await a11yAudit(axeConfig);
  });

  test('searching for "rust" from query', async function (assert) {
    this.server.loadFixtures();

    await visit('/search?q=rust');

    assert.equal(currentURL(), '/search?q=rust');
    assert.equal(getPageTitle(), "Search Results for 'rust' - crates.io: Rust Package Registry");

    assert.dom('[data-test-search-input]').hasValue('rust');
    assert.dom('[data-test-header]').hasText("Search Results for 'rust'");
    assert.dom('[data-test-search-nav]').hasText('Displaying 1-7 of 7 total results');
  });

  test('clearing search results', async function (assert) {
    this.server.loadFixtures();

    await visit('/search?q=rust');

    assert.equal(currentURL(), '/search?q=rust');
    assert.dom('[data-test-search-input]').hasValue('rust');

    await visit('/');

    assert.equal(currentURL(), '/');
    assert.dom('[data-test-search-input]').hasValue('');
  });

  test('pressing S key to focus the search bar', async function (assert) {
    this.server.loadFixtures();

    await visit('/');

    await blur('[data-test-search-input]');
    await keyDown('a');
    assert.dom('[data-test-search-input]').isNotFocused();

    await blur('[data-test-search-input]');
    await keyDown('s');
    assert.dom('[data-test-search-input]').isFocused();

    await blur('[data-test-search-input]');
    await keyDown('s');
    assert.dom('[data-test-search-input]').isFocused();

    await blur('[data-test-search-input]');
    await keyDown('S');
    assert.dom('[data-test-search-input]').isFocused();

    await blur('[data-test-search-input]');
    await keyDown('shift+s');
    assert.dom('[data-test-search-input]').isFocused();
  });

  test('check search results are by default displayed by relevance', async function (assert) {
    this.server.loadFixtures();

    await visit('/');
    await fillIn('[data-test-search-input]', 'rust');
    await triggerEvent('[data-test-search-form]', 'submit');

    assert.dom('[data-test-search-sort] [data-test-current-order]').hasText('Relevance');
  });

  test('error handling when searching from the frontpage', async function (assert) {
    let crate = this.server.create('crate', { name: 'rust' });
    this.server.create('version', { crate, num: '1.0.0' });

    this.server.get('/api/v1/crates', {}, 500);

    await visit('/');
    await fillIn('[data-test-search-input]', 'rust');
    await triggerEvent('[data-test-search-form]', 'submit');
    assert.dom('[data-test-crate-row]').doesNotExist();
    assert.dom('[data-test-error-message]').exists();
    assert.dom('[data-test-try-again-button]').isEnabled();

    let deferred = defer();
    this.server.get('/api/v1/crates', async function (schema, request) {
      await deferred.promise;
      return listCrates.call(this, schema, request);
    });

    click('[data-test-try-again-button]');
    await waitFor('[data-test-page-header] [data-test-spinner]');
    assert.dom('[data-test-crate-row]').doesNotExist();
    assert.dom('[data-test-error-message]').exists();
    assert.dom('[data-test-try-again-button]').isDisabled();

    deferred.resolve();
    await settled();
    assert.dom('[data-test-error-message]').doesNotExist();
    assert.dom('[data-test-try-again-button]').doesNotExist();
    assert.dom('[data-test-crate-row]').exists({ count: 1 });
  });

  test('error handling when searching from the search page', async function (assert) {
    let crate = this.server.create('crate', { name: 'rust' });
    this.server.create('version', { crate, num: '1.0.0' });

    await visit('/search?q=rust');
    assert.dom('[data-test-crate-row]').exists({ count: 1 });
    assert.dom('[data-test-error-message]').doesNotExist();
    assert.dom('[data-test-try-again-button]').doesNotExist();

    this.server.get('/api/v1/crates', {}, 500);

    await fillIn('[data-test-search-input]', 'ru');
    await triggerEvent('[data-test-search-form]', 'submit');
    assert.dom('[data-test-crate-row]').doesNotExist();
    assert.dom('[data-test-error-message]').exists();
    assert.dom('[data-test-try-again-button]').isEnabled();

    let deferred = defer();
    this.server.get('/api/v1/crates', async function (schema, request) {
      await deferred.promise;
      return listCrates.call(this, schema, request);
    });

    click('[data-test-try-again-button]');
    await waitFor('[data-test-page-header] [data-test-spinner]');
    assert.dom('[data-test-crate-row]').doesNotExist();
    assert.dom('[data-test-error-message]').exists();
    assert.dom('[data-test-try-again-button]').isDisabled();

    deferred.resolve();
    await settled();
    assert.dom('[data-test-crate-row]').exists({ count: 1 });
  });

  test('passes query parameters to the backend', async function (assert) {
    this.server.get('/api/v1/crates', function (schema, request) {
      assert.step('/api/v1/crates');

      assert.deepEqual(request.queryParams, {
        all_keywords: 'fire ball',
        page: '3',
        per_page: '15',
        q: 'rust',
        sort: 'new',
      });

      return { crates: [], meta: { total: 0 } };
    });

    await visit('/search?q=rust&page=3&per_page=15&sort=new&all_keywords=fire ball');
    assert.verifySteps(['/api/v1/crates']);
  });

  test('supports `keyword:bla` filters', async function (assert) {
    this.server.get('/api/v1/crates', function (schema, request) {
      assert.step('/api/v1/crates');

      assert.deepEqual(request.queryParams, {
        all_keywords: 'fire ball',
        page: '3',
        per_page: '15',
        q: 'rust',
        sort: 'new',
      });

      return { crates: [], meta: { total: 0 } };
    });

    await visit('/search?q=rust keyword:fire keyword:ball&page=3&per_page=15&sort=new');
    assert.verifySteps(['/api/v1/crates']);
  });

  test('`all_keywords` query parameter takes precedence over `keyword` filters', async function (assert) {
    this.server.get('/api/v1/crates', function (schema, request) {
      assert.step('/api/v1/crates');

      assert.deepEqual(request.queryParams, {
        all_keywords: 'fire ball',
        page: '3',
        per_page: '15',
        q: 'rust keywords:foo',
        sort: 'new',
      });

      return { crates: [], meta: { total: 0 } };
    });

    await visit('/search?q=rust keywords:foo&page=3&per_page=15&sort=new&all_keywords=fire ball');
    assert.verifySteps(['/api/v1/crates']);
  });
});
