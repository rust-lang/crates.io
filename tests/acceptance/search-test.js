import { blur, click, currentURL, fillIn, settled, triggerEvent, visit, waitFor } from '@ember/test-helpers';
import { module, test } from 'qunit';

import { defer } from 'rsvp';

import { loadFixtures } from '@crates-io/msw/fixtures.js';
import percySnapshot from '@percy/ember';
import a11yAudit from 'ember-a11y-testing/test-support/audit';
import { keyDown } from 'ember-keyboard/test-support/test-helpers';
import { getPageTitle } from 'ember-page-title/test-support';
import { http, HttpResponse } from 'msw';

import { setupApplicationTest } from 'crates-io/tests/helpers';

import axeConfig from '../axe-config';

module('Acceptance | search', function (hooks) {
  setupApplicationTest(hooks);

  test('searching for "rust"', async function (assert) {
    loadFixtures(this.db);

    await visit('/');
    await fillIn('[data-test-search-input]', 'rust');
    await triggerEvent('[data-test-search-form]', 'submit');

    assert.strictEqual(currentURL(), '/search?q=rust');
    assert.strictEqual(getPageTitle(), "Search Results for 'rust' - crates.io: Rust Package Registry");

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
    loadFixtures(this.db);

    await visit('/search?q=rust');

    assert.strictEqual(currentURL(), '/search?q=rust');
    assert.strictEqual(getPageTitle(), "Search Results for 'rust' - crates.io: Rust Package Registry");

    assert.dom('[data-test-search-input]').hasValue('rust');
    assert.dom('[data-test-header]').hasText("Search Results for 'rust'");
    assert.dom('[data-test-search-nav]').hasText('Displaying 1-7 of 7 total results');
  });

  test('clearing search results', async function (assert) {
    loadFixtures(this.db);

    await visit('/search?q=rust');

    assert.strictEqual(currentURL(), '/search?q=rust');
    assert.dom('[data-test-search-input]').hasValue('rust');

    await visit('/');

    assert.strictEqual(currentURL(), '/');
    assert.dom('[data-test-search-input]').hasValue('');
  });

  test('pressing S key to focus the search bar', async function (assert) {
    loadFixtures(this.db);

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
    loadFixtures(this.db);

    await visit('/');
    await fillIn('[data-test-search-input]', 'rust');
    await triggerEvent('[data-test-search-form]', 'submit');

    assert.dom('[data-test-search-sort] [data-test-current-order]').hasText('Relevance');
  });

  test('error handling when searching from the frontpage', async function (assert) {
    let crate = this.db.crate.create({ name: 'rust' });
    this.db.version.create({ crate, num: '1.0.0' });

    let error = HttpResponse.json({}, { status: 500 });
    this.worker.use(http.get('/api/v1/crates', () => error));

    await visit('/');
    await fillIn('[data-test-search-input]', 'rust');
    await triggerEvent('[data-test-search-form]', 'submit');
    assert.dom('[data-test-crate-row]').doesNotExist();
    assert.dom('[data-test-error-message]').exists();
    assert.dom('[data-test-try-again-button]').isEnabled();

    let deferred = defer();
    this.worker.resetHandlers();
    this.worker.use(http.get('/api/v1/crates', () => deferred.promise));

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
    let crate = this.db.crate.create({ name: 'rust' });
    this.db.version.create({ crate, num: '1.0.0' });

    await visit('/search?q=rust');
    assert.dom('[data-test-crate-row]').exists({ count: 1 });
    assert.dom('[data-test-error-message]').doesNotExist();
    assert.dom('[data-test-try-again-button]').doesNotExist();

    let error = HttpResponse.json({}, { status: 500 });
    this.worker.use(http.get('/api/v1/crates', () => error));

    await fillIn('[data-test-search-input]', 'ru');
    await triggerEvent('[data-test-search-form]', 'submit');
    assert.dom('[data-test-crate-row]').doesNotExist();
    assert.dom('[data-test-error-message]').exists();
    assert.dom('[data-test-try-again-button]').isEnabled();

    let deferred = defer();
    this.worker.resetHandlers();
    this.worker.use(http.get('/api/v1/crates', () => deferred.promise));

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
    this.worker.use(
      http.get('/api/v1/crates', function ({ request }) {
        assert.step('/api/v1/crates');

        let url = new URL(request.url);
        assert.deepEqual(Object.fromEntries(url.searchParams.entries()), {
          all_keywords: 'fire ball',
          page: '3',
          per_page: '15',
          q: 'rust',
          sort: 'new',
        });

        return HttpResponse.json({ crates: [], meta: { total: 0 } });
      }),
    );

    await visit('/search?q=rust&page=3&per_page=15&sort=new&all_keywords=fire ball');
    assert.verifySteps(['/api/v1/crates']);
  });

  test('supports `keyword:bla` filters', async function (assert) {
    this.worker.use(
      http.get('/api/v1/crates', function ({ request }) {
        assert.step('/api/v1/crates');

        let url = new URL(request.url);
        assert.deepEqual(Object.fromEntries(url.searchParams.entries()), {
          all_keywords: 'fire ball',
          page: '3',
          per_page: '15',
          q: 'rust',
          sort: 'new',
        });

        return HttpResponse.json({ crates: [], meta: { total: 0 } });
      }),
    );

    await visit('/search?q=rust keyword:fire keyword:ball&page=3&per_page=15&sort=new');
    assert.verifySteps(['/api/v1/crates']);
  });

  test('`all_keywords` query parameter takes precedence over `keyword` filters', async function (assert) {
    this.worker.use(
      http.get('/api/v1/crates', function ({ request }) {
        assert.step('/api/v1/crates');

        let url = new URL(request.url);
        assert.deepEqual(Object.fromEntries(url.searchParams.entries()), {
          all_keywords: 'fire ball',
          page: '3',
          per_page: '15',
          q: 'rust keywords:foo',
          sort: 'new',
        });

        return HttpResponse.json({ crates: [], meta: { total: 0 } });
      }),
    );

    await visit('/search?q=rust keywords:foo&page=3&per_page=15&sort=new&all_keywords=fire ball');
    assert.verifySteps(['/api/v1/crates']);
  });

  test('visiting without query parameters works', async function (assert) {
    loadFixtures(this.db);

    await visit('/search');

    assert.strictEqual(currentURL(), '/search');
    assert.strictEqual(getPageTitle(), 'Search Results - crates.io: Rust Package Registry');

    assert.dom('[data-test-header]').hasText('Search Results');
    assert.dom('[data-test-search-nav]').hasText('Displaying 1-10 of 23 total results');
    assert.dom('[data-test-crate-row="0"] [data-test-crate-link]').hasText('kinetic-rust');
    assert.dom('[data-test-crate-row="0"] [data-test-version]').hasText('v0.0.16');
  });
});
