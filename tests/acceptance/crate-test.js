import { test } from 'qunit';
import moduleForAcceptance from 'cargo/tests/helpers/module-for-acceptance';
import matchesText from 'cargo/tests/helpers/matches-text';
import hasText from 'cargo/tests/helpers/has-text';

moduleForAcceptance('Acceptance | crate page');

test('visiting a crate page from the front page', async function(assert) {
    await visit('/');
    await click('#just-updated ul > li:first a');

    assert.equal(currentURL(), '/crates/nanomsg');
    assert.equal(document.title, 'nanomsg - Cargo: packages for Rust');
});

test('visiting /crates/nanomsg', async function(assert) {
    await visit('/crates/nanomsg');

    assert.equal(currentURL(), '/crates/nanomsg');
    assert.equal(currentRouteName(), 'crate.index');
    assert.equal(document.title, 'nanomsg - Cargo: packages for Rust');

    assert.equal(find('#crates-heading .info h1').text(), 'nanomsg');
    assert.equal(find('#crates-heading .info h2').text(), '0.6.1');
});

test('visiting /crates/nanomsg/', async function(assert) {
    await visit('/crates/nanomsg/');

    assert.equal(currentURL(), '/crates/nanomsg/');
    assert.equal(currentRouteName(), 'crate.index');
    assert.equal(document.title, 'nanomsg - Cargo: packages for Rust');

    assert.equal(find('#crates-heading .info h1').text(), 'nanomsg');
    assert.equal(find('#crates-heading .info h2').text(), '0.6.1');
});

test('visiting /crates/nanomsg/0.6.0', async function(assert) {
    await visit('/crates/nanomsg/0.6.0');

    assert.equal(currentURL(), '/crates/nanomsg/0.6.0');
    assert.equal(currentRouteName(), 'crate.version');
    assert.equal(document.title, 'nanomsg - Cargo: packages for Rust');

    assert.equal(find('#crates-heading .info h1').text(), 'nanomsg');
    assert.equal(find('#crates-heading .info h2').text(), '0.6.0');
});

test('navigating to the all versions page', async function(assert) {
    await visit('/crates/nanomsg');
    await click('#crate-versions span.small a');

    matchesText(assert, '.info', /All 13 versions of nanomsg since December \d+, 2014/);
});

test('navigating to the reverse dependencies page', async function(assert) {
    await visit('/crates/nanomsg');
    await click('a:contains("Dependent crates")');

    assert.equal(currentURL(), '/crates/nanomsg/reverse_dependencies');

    const $revDep = findWithAssert('a[href="/crates/unicorn-rpc"]:first');

    hasText(assert, $revDep, 'unicorn-rpc');
});

test('navigating to a user page', function(assert) {
    visit('/crates/nanomsg');
    click('.owners li:last a');

    andThen(function() {
        assert.equal(currentURL(), '/users/blabaere');
        hasText(assert, '#crates-heading h1', 'thehydroimpulse');
    });
});

test('navigating to a team page', function(assert) {
    visit('/crates/nanomsg');
    click('.owners li:first a ');

    andThen(function() {
        assert.equal(currentURL(), '/teams/github:org:thehydroimpulse');
        hasText(assert, '.team-info h2', 'thehydroimpulseteam');
    });
});

test('crates having user-owners', function(assert) {
    visit('/crates/nanomsg');

    andThen(function() {
        findWithAssert('ul.owners li:first a[href="/teams/github:org:thehydroimpulse"] img[src="https://avatars.githubusercontent.com/u/565790?v=3&s=64"]');
        assert.equal(find('ul.owners li').length, 4);
    });
});

test('crates having team-owners', function(assert) {
    visit('/crates/nanomsg');

    andThen(function() {
        findWithAssert('ul.owners li:first a[href="/teams/github:org:thehydroimpulse"]');
        assert.equal(find('ul.owners li').length, 4);
    });
});
