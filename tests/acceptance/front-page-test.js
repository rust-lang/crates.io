import { test } from 'qunit';
import moduleForAcceptance from 'cargo/tests/helpers/module-for-acceptance';

moduleForAcceptance('Acceptance | front page');

test('visiting /', async function(assert) {
    server.loadFixtures();

    await visit('/');

    assert.equal(currentURL(), '/');
    assert.equal(document.title, 'Cargo: packages for Rust');

    assert.dom('a[href="/install"]').exists();
    assert.dom('a[href="/crates"]').exists();
    assert.dom('a[href="/login"]').exists();

    assert.dom('.downloads .num').hasText('122,669');
    assert.dom('.crates .num').hasText('19');

    assert.dom('#new-crates ul > li a').hasText('Inflector (0.1.6)');
    assert.dom('#new-crates ul > li a').hasAttribute('href', '/crates/Inflector');

    assert.dom('#most-downloaded ul > li a').hasText('serde (0.6.1)');
    assert.dom('#most-downloaded ul > li a').hasAttribute('href', '/crates/serde');

    assert.dom('#just-updated ul > li a').hasText('nanomsg (0.7.0-alpha)');
    assert.dom('#just-updated ul > li a').hasAttribute('href', '/crates/nanomsg');
});
