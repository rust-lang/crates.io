import { test } from 'qunit';
import moduleForAcceptance from 'cargo/tests/helpers/module-for-acceptance';

moduleForAcceptance('Acceptance | search');

test('searching for "rust"', async function(assert) {
    server.loadFixtures();

    await visit('/');
    await fillIn('input.search', 'rust');

    findWithAssert('form.search').submit();

    await wait();

    assert.equal(currentURL(), '/search?q=rust');
    assert.equal(document.title, 'Search Results for \'rust\' - Cargo: packages for Rust');

    assert.dom('#crates-heading').hasText('Search Results for \'rust\'');
    assert.dom('#results').hasText('Displaying 1-8 of 8 total results Sort by Relevance Relevance All-Time Downloads Recent Downloads');
    assert.dom('#crates .row .desc .info').hasText('kinetic-rust');
    assert.dom('#crates .row .desc .info .vers img[alt="0.0.16"]').exists();

    assert.dom('#crates .row .desc .summary').hasText('A Kinetic protocol library written in Rust');
    assert.dom('#crates .row .downloads').hasText('All-Time: 225');
    assert.dom('#crates .row .desc .info img[alt="Maintenance intention for this crate"]').exists();
});

test('pressing S key to focus the search bar', async function(assert) {
    server.loadFixtures();

    const KEYCODE_S = 83;
    const KEYCODE_A = 65;

    function assertSearchBarIsFocused() {
        assert.dom('#cargo-desktop-search').isFocused();
        find('#cargo-desktop-search').blur();
    }

    await visit('/');

    findWithAssert('#cargo-desktop-search').blur();

    await keyEvent(document, 'keypress', KEYCODE_A);
    assert.dom('#cargo-desktop-search').isNotFocused();
    find('#cargo-desktop-search').blur();

    await keyEvent(document, 'keypress', KEYCODE_S);
    assertSearchBarIsFocused();

    await keyEvent(document, 'keydown', KEYCODE_S);
    assertSearchBarIsFocused();
});

test('check search results are by default displayed by relevance', async function(assert) {
    server.loadFixtures();

    await visit('/');
    await fillIn('input.search', 'rust');

    findWithAssert('form.search').submit();

    await wait();

    assert.dom('div.sort div.dropdown-container a.dropdown').hasText('Relevance');
});
