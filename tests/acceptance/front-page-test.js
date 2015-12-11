import { test } from 'qunit';
import moduleForAcceptance from 'cargo/tests/helpers/module-for-acceptance';

moduleForAcceptance('Acceptance | front page');

test('visiting /', function(assert) {
    visit('/');

    andThen(function() {
        assert.equal(currentURL(), '/');
        assert.equal(document.title, 'Cargo');

        findWithAssert('a[href="/install"]');
        findWithAssert('a[href="/crates"]');
        findWithAssert('a[href="/login"]');

        hasText(assert, '.downloads .num', '13,534,453');
        hasText(assert, '.crates .num', '3,430');

        const $newCrate = findWithAssert('#new-crates ul > li:first a');
        hasText(assert, $newCrate, 'mkstemp (0.2.0)');
        assert.equal($newCrate.attr('href').trim(), '/crates/mkstemp');

        const $mostDownloaded = findWithAssert('#most-downloaded ul > li:first a');
        hasText(assert, $mostDownloaded, 'libc (0.2.2)');
        assert.equal($mostDownloaded.attr('href').trim(), '/crates/libc');

        const $justUpdated = findWithAssert('#just-updated ul > li:first a');
        hasText(assert, $justUpdated, 'nanomsg (0.4.2)');
        assert.equal($justUpdated.attr('href').trim(), '/crates/nanomsg');
    });
});
