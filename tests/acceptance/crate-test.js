import { test } from 'qunit';
import moduleForAcceptance from 'cargo/tests/helpers/module-for-acceptance';

moduleForAcceptance('Acceptance | crate page');

test('visiting a crate page from the front page', async function(assert) {
    server.create('crate', 'withVersion', { id: 'nanomsg' });

    await visit('/');
    await click('#just-updated ul > li:first a');

    assert.equal(currentURL(), '/crates/nanomsg');
    assert.equal(document.title, 'nanomsg - Cargo: packages for Rust');
});

test('visiting /crates/nanomsg', async function(assert) {
    server.create('crate', { id: 'nanomsg', max_version: '0.6.1' });
    server.create('version', { crate: 'nanomsg', num: '0.6.0' });
    server.create('version', { crate: 'nanomsg', num: '0.6.1' });

    await visit('/crates/nanomsg');

    assert.equal(currentURL(), '/crates/nanomsg');
    assert.equal(currentRouteName(), 'crate.index');
    assert.equal(document.title, 'nanomsg - Cargo: packages for Rust');

    assert.dom('#crates-heading .info h1').hasText('nanomsg');
    assert.dom('#crates-heading .info h2').hasText('0.6.1');
});

test('visiting /crates/nanomsg/', async function(assert) {
    server.create('crate', { id: 'nanomsg', max_version: '0.6.1' });
    server.create('version', { crate: 'nanomsg', num: '0.6.0' });
    server.create('version', { crate: 'nanomsg', num: '0.6.1' });

    await visit('/crates/nanomsg/');

    assert.equal(currentURL(), '/crates/nanomsg/');
    assert.equal(currentRouteName(), 'crate.index');
    assert.equal(document.title, 'nanomsg - Cargo: packages for Rust');

    assert.dom('#crates-heading .info h1').hasText('nanomsg');
    assert.dom('#crates-heading .info h2').hasText('0.6.1');
});

test('visiting /crates/nanomsg/0.6.0', async function(assert) {
    server.create('crate', { id: 'nanomsg', max_version: '0.6.1' });
    server.create('version', { crate: 'nanomsg', num: '0.6.0' });
    server.create('version', { crate: 'nanomsg', num: '0.6.1' });

    await visit('/crates/nanomsg/0.6.0');

    assert.equal(currentURL(), '/crates/nanomsg/0.6.0');
    assert.equal(currentRouteName(), 'crate.version');
    assert.equal(document.title, 'nanomsg - Cargo: packages for Rust');

    assert.dom('#crates-heading .info h1').hasText('nanomsg');
    assert.dom('#crates-heading .info h2').hasText('0.6.0');
});

test('navigating to the all versions page', async function(assert) {
    server.loadFixtures();

    await visit('/crates/nanomsg');
    await click('#crate-versions span.small a');

    assert.dom('.info').hasText(/All 13\s+versions of nanomsg since\s+December \d+, 2014/);
});

test('navigating to the reverse dependencies page', async function(assert) {
    server.loadFixtures();

    await visit('/crates/nanomsg');
    await click('a:contains("Dependent crates")');

    assert.equal(currentURL(), '/crates/nanomsg/reverse_dependencies');
    assert.dom('a[href="/crates/unicorn-rpc"]').hasText('unicorn-rpc');
});

test('navigating to a user page', async function(assert) {
    server.loadFixtures();

    await visit('/crates/nanomsg');
    await click('.owners li:last a');

    assert.equal(currentURL(), '/users/blabaere');
    assert.dom('#crates-heading h1').hasText('blabaere');
});

test('navigating to a team page', async function(assert) {
    server.loadFixtures();

    await visit('/crates/nanomsg');
    await click('.owners li:first a ');

    assert.equal(currentURL(), '/teams/github:org:thehydroimpulse');
    assert.dom('.team-info h2').hasText('thehydroimpulseteam');
});

test('crates having user-owners', async function(assert) {
    server.loadFixtures();

    await visit('/crates/nanomsg');

    assert.dom('ul.owners li a[href="/teams/github:org:thehydroimpulse"] img[src="https://avatars.githubusercontent.com/u/565790?v=3&s=64"]').exists();
    assert.dom('ul.owners li').exists({ count: 4 });
});

test('crates having team-owners', async function(assert) {
    server.loadFixtures();

    await visit('/crates/nanomsg');

    assert.dom('ul.owners li a[href="/teams/github:org:thehydroimpulse"]').exists();
    assert.dom('ul.owners li').exists({ count: 4 });
});

test('crates license is supplied by version', async function(assert) {
    server.loadFixtures();

    await visit('/crates/nanomsg');
    assert.dom('.license').hasText('Apache-2.0');

    await click('#crate-versions a:contains("0.5.0")');
    assert.dom('.license').hasText('MIT/Apache-2.0');
});
