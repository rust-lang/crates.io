/*global hasText, matchesText*/
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
        matchesText(assert, '.info', /All 13 versions of nanomsg since December \d+, 2014/);
    });
});

test('navigating to the reverse dependencies page', function(assert) {
    visit('/crates/nanomsg');
    click('a:contains("Dependent crates")');

    andThen(function() {
        assert.equal(currentURL(), '/crates/nanomsg/reverse_dependencies');

        const $revDep = findWithAssert('a[href="/crates/unicorn-rpc"]:first');

        hasText(assert, $revDep, 'unicorn-rpc');
    });
});
