import { test } from 'qunit';
import moduleForAcceptance from 'cargo/tests/helpers/module-for-acceptance';
import hasText from 'cargo/tests/helpers/has-text';

moduleForAcceptance('Acceptance | front page');

test('visiting /', async function(assert) {
    server.loadFixtures();

    await visit('/');

    assert.equal(currentURL(), '/');
    assert.equal(document.title, 'Cargo: packages for Rust');

    findWithAssert('a[href="/install"]');
    findWithAssert('a[href="/crates"]');
    findWithAssert('a[href="/login"]');

    hasText(assert, '.downloads .num', '122,669');
    hasText(assert, '.crates .num', '19');

    const $newCrate = findWithAssert('#new-crates ul > li:first a');
    hasText(assert, $newCrate, 'Inflector (0.1.6)');
    assert.equal($newCrate.attr('href').trim(), '/crates/Inflector');

    const $mostDownloaded = findWithAssert('#most-downloaded ul > li:first a');
    hasText(assert, $mostDownloaded, 'serde (0.6.1)');
    assert.equal($mostDownloaded.attr('href').trim(), '/crates/serde');

    const $justUpdated = findWithAssert('#just-updated ul > li:first a');
    hasText(assert, $justUpdated, 'nanomsg (0.7.0-alpha)');
    assert.equal($justUpdated.attr('href').trim(), '/crates/nanomsg');
});
