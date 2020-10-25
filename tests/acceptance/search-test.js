import { fillIn, currentURL, triggerEvent, visit, blur } from '@ember/test-helpers';
import { setupApplicationTest } from 'ember-qunit';
import { module, test } from 'qunit';

import percySnapshot from '@percy/ember';
import a11yAudit from 'ember-a11y-testing/test-support/audit';
import { keyDown } from 'ember-keyboard/test-support/test-helpers';

import axeConfig from '../axe-config';
import { title } from '../helpers/dom';
import setupMirage from '../helpers/setup-mirage';

module('Acceptance | search', function (hooks) {
  setupApplicationTest(hooks);
  setupMirage(hooks);

  test('searching for "rust"', async function (assert) {
    this.server.loadFixtures();

    await visit('/');
    await fillIn('[data-test-search-input]', 'rust');
    await triggerEvent('[data-test-search-form]', 'submit');

    assert.equal(currentURL(), '/search?q=rust');
    assert.equal(title(), "Search Results for 'rust' - crates.io: Rust Package Registry");

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
    assert.equal(title(), "Search Results for 'rust' - crates.io: Rust Package Registry");

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
});
