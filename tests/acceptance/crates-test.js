import { test } from 'qunit';
import moduleForAcceptance from 'cargo/tests/helpers/module-for-acceptance';
import hasText from 'cargo/tests/helpers/has-text';

moduleForAcceptance('Acceptance | crates page');

test('visiting the crates page from the front page', async function(assert) {
    server.loadFixtures();

    await visit('/');
    await click('a[href="/crates"]');

    assert.equal(currentURL(), '/crates');
    assert.equal(document.title, 'Crates - Cargo: packages for Rust');
});

test('visiting the crates page directly', async function(assert) {
    server.loadFixtures();

    await visit('/crates');
    await click('a[href="/crates"]');

    assert.equal(currentURL(), '/crates');
    assert.equal(document.title, 'Crates - Cargo: packages for Rust');
});

test('listing crates', async function(assert) {
    server.loadFixtures();

    await visit('/crates');

    hasText(assert, '.amt.small .cur', '1-10');
    hasText(assert, '.amt.small .total', '19');
});

test('navigating to next page of crates', async function(assert) {
    server.loadFixtures();

    await visit('/crates');
    await click('.pagination .next');

    assert.equal(currentURL(), '/crates?page=2');
    hasText(assert, '.amt.small .cur', '11-19');
    hasText(assert, '.amt.small .total', '19');
});

test('crates default sort is alphabetical', async function(assert) {
    server.loadFixtures();

    await visit('/crates');

    const $sort = findWithAssert('div.sort div.dropdown-container a.dropdown');
    hasText(assert, $sort, 'Alphabetical');
});

test('downloads appears for each crate on crate list', async function(assert) {
    server.loadFixtures();

    await visit('/crates');
    const $recentDownloads = findWithAssert('div.downloads:first span.num');
    hasText(assert, $recentDownloads, 'All-Time: 497');
});

test('recent downloads appears for each crate on crate list', async function(assert) {
    server.loadFixtures();

    await visit('/crates');
    const $recentDownloads = findWithAssert('div.recent-downloads:first span.num');
    hasText(assert, $recentDownloads, 'Recent:');
});
