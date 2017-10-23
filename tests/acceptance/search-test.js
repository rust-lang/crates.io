import { test } from 'qunit';
import { fillIn, keyEvent, visit, triggerEvent, currentURL, blur } from 'ember-native-dom-helpers';
import moduleForAcceptance from 'cargo/tests/helpers/module-for-acceptance';

moduleForAcceptance('Acceptance | search');

test('searching for "rust"', async function(assert) {
    server.loadFixtures();

    await visit('/');
    await fillIn('[data-test-search-input]', 'rust');
    await triggerEvent('[data-test-search-form]', 'submit');

    assert.equal(currentURL(), '/search?q=rust');
    assert.equal(document.title, 'Search Results for \'rust\' - Cargo: packages for Rust');

    assert.dom('[data-test-heading]')
        .hasText('Search Results for \'rust\'');
    assert.dom('[data-test-search-nav]').hasText('Displaying 1-8 of 8 total results');
    assert.dom('[data-test-search-sort]').hasText('Sort by Relevance Relevance All-Time Downloads Recent Downloads');
    assert.dom('[data-test-crate-row="0"] [data-test-crate-link]').hasText('kinetic-rust');
    assert.dom('[data-test-crate-row="0"] [data-test-version-badge]').hasAttribute('alt', '0.0.16');

    assert.dom('[data-test-crate-row="0"] [data-test-description]').hasText('A Kinetic protocol library written in Rust');
    assert.dom('[data-test-crate-row="0"] [data-test-downloads]').hasText('All-Time: 225');
    assert.dom('[data-test-crate-row="0"] [data-test-badge="maintenance"]').exists();
});

test('pressing S key to focus the search bar', async function(assert) {
    server.loadFixtures();

    await visit('/');

    await blur('[data-test-search-input]');
    await keyEvent(document, 'keypress', 'a');
    assert.dom('[data-test-search-input]').isNotFocused();

    await blur('[data-test-search-input]');
    await keyEvent(document, 'keypress', 's');
    assert.dom('[data-test-search-input]').isFocused();

    await blur('[data-test-search-input]');
    await keyEvent(document, 'keydown', 's');
    assert.dom('[data-test-search-input]').isFocused();
});

test('check search results are by default displayed by relevance', async function(assert) {
    server.loadFixtures();

    await visit('/');
    await fillIn('[data-test-search-input]', 'rust');
    await triggerEvent('[data-test-search-form]', 'submit');

    assert.dom('[data-test-search-sort] [data-test-current-order]').hasText('Relevance');
});
