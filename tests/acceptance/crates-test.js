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
