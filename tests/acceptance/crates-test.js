import { test } from 'qunit';
import moduleForAcceptance from 'cargo/tests/helpers/module-for-acceptance';

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

    assert.dom('.amt.small .cur').hasText('1-10');
    assert.dom('.amt.small .total').hasText('19');
});

test('navigating to next page of crates', async function(assert) {
    server.loadFixtures();

    await visit('/crates');
    await click('.pagination .next');

    assert.equal(currentURL(), '/crates?page=2');
    assert.dom('.amt.small .cur').hasText('11-19');
    assert.dom('.amt.small .total').hasText('19');
});

test('crates default sort is alphabetical', async function(assert) {
    server.loadFixtures();

    await visit('/crates');

    assert.dom('div.sort div.dropdown-container a.dropdown').hasText('Alphabetical');
});

test('downloads appears for each crate on crate list', async function(assert) {
    server.loadFixtures();

    await visit('/crates');
    assert.dom('div.downloads span.num').hasText('All-Time: 497');
});

test('recent downloads appears for each crate on crate list', async function(assert) {
    server.loadFixtures();

    await visit('/crates');
    assert.dom('div.recent-downloads span.num').hasText('Recent: 497');
});
