import { test } from 'qunit';
import moduleForAcceptance from 'cargo/tests/helpers/module-for-acceptance';

moduleForAcceptance('Acceptance | search');

test('searching for "rust"', async function(assert) {
    await visit('/');
    await fillIn('input.search', 'rust');

    findWithAssert('form.search').submit();

    await wait();

    assert.equal(currentURL(), '/search?q=rust');
    assert.equal(document.title, 'Search Results for \'rust\' - Cargo: packages for Rust');

    findWithAssert('a[href="/search?page=2&q=rust"]');
    assert.notOk(find('a[href="/search?page=3&q=rust"]')[0]);

    hasText(assert, '#crates-heading', 'Search Results for \'rust\'');
    hasText(assert, '#results', 'Displaying 1-10 of 18 total results Sort by Relevance Relevance Downloads');

    hasText(assert, '#crates .row:first .desc .info', 'rust_mixin');
    findWithAssert('#crates .row:first .desc .info .vers img[alt="0.0.1"]');

    findWithAssert('#crates .row:first .desc .info .badge:first a[href="https://ci.appveyor.com/project/huonw/external_mixin"]');
    findWithAssert('#crates .row:first .desc .info .badge:first a img[src="https://ci.appveyor.com/api/projects/status/github/huonw/external_mixin?svg=true&branch=master"]');
    findWithAssert('#crates .row:first .desc .info .badge:eq(1) a[href="https://codecov.io/github/huonw/external_mixin?branch=master"]');
    findWithAssert('#crates .row:first .desc .info .badge:eq(1) a img[src="https://codecov.io/github/huonw/external_mixin/coverage.svg?branch=master"]');
    findWithAssert('#crates .row:first .desc .info .badge:eq(2) a[href="https://coveralls.io/github/huonw/external_mixin?branch=master"]');
    findWithAssert('#crates .row:first .desc .info .badge:eq(2) a img[src="https://coveralls.io/repos/github/huonw/external_mixin/badge.svg?branch=master"]');
    findWithAssert('#crates .row:first .desc .info .badge:eq(3) a[href="https://gitlab.com/huonw/external_mixin/pipelines"]');
    findWithAssert('#crates .row:first .desc .info .badge:eq(3) a img[src="https://gitlab.com/huonw/external_mixin/badges/master/build.svg"]');
    findWithAssert('#crates .row:first .desc .info .badge:eq(4) a[href="https://isitmaintained.com/project/huonw/external_mixin"]');
    findWithAssert('#crates .row:first .desc .info .badge:eq(4) a img[src="https://isitmaintained.com/badge/resolution/huonw/external_mixin.svg"]');
    findWithAssert('#crates .row:first .desc .info .badge:eq(5) a[href="https://isitmaintained.com/project/huonw/external_mixin"]');
    findWithAssert('#crates .row:first .desc .info .badge:eq(5) a img[src="https://isitmaintained.com/badge/open/huonw/external_mixin.svg"]');
    findWithAssert('#crates .row:first .desc .info .badge:eq(6) a[href="https://travis-ci.org/huonw/external_mixin"]');
    findWithAssert('#crates .row:first .desc .info .badge:eq(6) a img[src="https://travis-ci.org/huonw/external_mixin.svg?branch=master"]');

    hasText(assert, '#crates .row:first .desc .summary', 'Yo dawg, use Rust to generate Rust, right in your Rust. (See `external_mixin` to use scripting languages.)');
    hasText(assert, '#crates .row:first .downloads', '477');

    await click('a[href="/search?page=2&q=rust"]');

    assert.equal(currentURL(), '/search?page=2&q=rust');
    assert.equal(document.title, 'Search Results for \'rust\' - Cargo: packages for Rust');

    findWithAssert('a[href="/search?q=rust"]');
    assert.notOk(find('a[href="/search?page=3&q=rust"]')[0]);

    hasText(assert, '#crates-heading', 'Search Results for \'rust\'');
    hasText(assert, '#results', 'Displaying 11-18 of 18 total results Sort by Relevance Relevance Downloads');

    hasText(assert, '#crates .row:first .desc .info', 'rusted_cypher');
    findWithAssert('#crates .row:first .desc .info .vers img[alt="0.7.1"]');
});

test('pressing S key to focus the search bar', async function(assert) {
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
