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

        const $downloads = findWithAssert('.downloads .num');
        assert.equal($downloads.text().trim(), '13,534,453');

        const $crates = findWithAssert('.crates .num');
        assert.equal($crates.text().trim(), '3,430');

        const $newCrate = findWithAssert('#new-crates ul > li:first a');
        assert.equal($newCrate.text().trim(), 'mkstemp (0.2.0)');
        assert.equal($newCrate.attr('href').trim(), '/crates/mkstemp');

        const $mostDownloaded = findWithAssert('#most-downloaded ul > li:first a');
        assert.equal($mostDownloaded.text().trim(), 'libc (0.2.2)');
        assert.equal($mostDownloaded.attr('href').trim(), '/crates/libc');

        const $justUpdated = findWithAssert('#just-updated ul > li:first a');
        assert.equal($justUpdated.text().trim(), 'nanomsg (0.4.2)');
        assert.equal($justUpdated.attr('href').trim(), '/crates/nanomsg');
    });
});
