import { test } from 'qunit';
import moduleForAcceptance from 'cargo/tests/helpers/module-for-acceptance';
import hasText from 'cargo/tests/helpers/has-text';

moduleForAcceptance('Acceptance | search');

test('searching for "rust"', async function(assert) {
    server.loadFixtures();

    await visit('/');
    await fillIn('input.search', 'rust');

    findWithAssert('form.search').submit();

    await wait();

    assert.equal(currentURL(), '/search?q=rust');
    assert.equal(document.title, 'Search Results for \'rust\' - Cargo: packages for Rust');

    hasText(assert, '#crates-heading', 'Search Results for \'rust\'');
    hasText(assert, '#results', 'Displaying 1-8 of 8 total results Sort by Relevance Relevance All-Time Downloads Recent Downloads');

    hasText(assert, '#crates .row:first .desc .info', 'kinetic-rust');
    findWithAssert('#crates .row:first .desc .info .vers img[alt="0.0.16"]');

    hasText(assert, '#crates .row:first .desc .summary', 'A Kinetic protocol library written in Rust');
    hasText(assert, '#crates .row:first .downloads', 'All-Time: 225');
    findWithAssert('#crates .row:first .desc .info img[alt="Maintenance intention for this crate"]');
});

test('pressing S key to focus the search bar', async function(assert) {
    server.loadFixtures();

    const KEYCODE_S = 83;
    const KEYCODE_A = 65;

    function assertSearchBarIsFocused() {
        const $searchBar = find('#cargo-desktop-search');
        assert.equal($searchBar[0], document.activeElement);
        $searchBar.blur();
    }

    await visit('/');

    findWithAssert('#cargo-desktop-search').blur();

    await keyEvent(document, 'keypress', KEYCODE_A);

    const $searchBar = find('#cargo-desktop-search');
    assert.notEqual($searchBar[0], document.activeElement);
    $searchBar.blur();

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

    const $sort = findWithAssert('div.sort div.dropdown-container a.dropdown');
    hasText(assert, $sort, 'Relevance');
});
