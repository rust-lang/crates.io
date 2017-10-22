import { test } from 'qunit';
import moduleForAcceptance from 'cargo/tests/helpers/module-for-acceptance';
import Ember from 'ember';

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

test('navigating to the owners page when not logged in', async function(assert) {
    server.loadFixtures();

    await visit('/crates/nanomsg');

    assert.dom('#crate-owners p a').doesNotExist();
});

test('navigating to the owners page when not an owner', async function(assert) {
    server.loadFixtures();

    this.application.register('service:session-b', Ember.Service.extend({
        currentUser: {
            id: 'iain8'
        }
    }));

    this.application.inject('controller', 'session', 'service:session-b');

    await visit('/crates/nanomsg');

    assert.dom('#crate-owners p a').doesNotExist();
});

test('navigating to the owners page', async function(assert) {
    server.loadFixtures();

    this.application.register('service:session-b', Ember.Service.extend({
        currentUser: {
            id: 'thehydroimpulse'
        }
    }));

    this.application.inject('controller', 'session', 'service:session-b');

    await visit('/crates/nanomsg');
    await click('#crate-owners p a');

    assert.dom('#crates-heading h1').hasText('Manage Crate Owners');
});

test('listing crate owners', async function(assert) {
    server.loadFixtures();

    await visit('/crates/nanomsg/owners');

    assert.dom('.owners .row').exists({ count: 2 });
    assert.dom('a[href="/users/thehydroimpulse"]').exists();
    assert.dom('a[href="/users/blabaere"]').exists();
});

test('attempting to add owner without username', async function(assert) {
    server.loadFixtures();

    await visit('/crates/nanomsg/owners');
    await click('#add-owner');

    assert.dom('.error').exists();
    assert.dom('.error').hasText('Please enter a username');
    assert.dom('.owners .row').exists({ count: 2 });
});

test('attempting to add non-existent owner', async function(assert) {
    server.loadFixtures();

    await visit('/crates/nanomsg/owners');
    await fillIn('input[name="username"]', 'spookyghostboo');
    await click('#add-owner');

    assert.dom('.error').exists();
    assert.dom('.error').hasText('Error sending invite');
    assert.dom('.owners .row').exists({ count: 2 });
});

test('add a new owner', async function(assert) {
    server.loadFixtures();

    await visit('/crates/nanomsg/owners');
    await fillIn('input[name="username"]', 'iain8');
    await click('#add-owner');

    assert.dom('.invited').exists();
    assert.dom('.invited').hasText('An invite has been sent to iain8');
    assert.dom('.owners .row').exists({ count: 2 });
});

test('remove a crate owner', async function(assert) {
    server.loadFixtures();

    await visit('/crates/nanomsg/owners');
    await click('.owners .row:first-child .remove-owner');

    assert.dom('.removed').exists();
    assert.dom('.owners .row').exists({ count: 1 });
});
