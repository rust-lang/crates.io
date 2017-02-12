import { test } from 'qunit';
import moduleForAcceptance from 'cargo/tests/helpers/module-for-acceptance';

moduleForAcceptance('Acceptance | crate page');

test('visiting a crate page from the front page', function(assert) {
    visit('/');
    click('#just-updated ul > li:first a');

    andThen(function() {
        assert.equal(currentURL(), '/crates/nanomsg');
        assert.equal(document.title, 'nanomsg - Cargo: packages for Rust');
    });
});

test('visiting a crate page directly', function(assert) {
    visit('/crates/nanomsg');

    andThen(function() {
        assert.equal(currentURL(), '/crates/nanomsg');
        assert.equal(document.title, 'nanomsg - Cargo: packages for Rust');
    });
});

test('navigating to the all versions page', function(assert) {
    visit('/crates/nanomsg');
    click('#crate-versions span.small a');

    andThen(function() {
        matchesText(assert, '.info', /All 12 versions of nanomsg since December \d+, 2014/);
    });
});
