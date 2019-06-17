import Service from '@ember/service';
import { module, test } from 'qunit';
import { setupApplicationTest } from 'ember-qunit';
import { click, fillIn, currentURL, currentRouteName, visit } from '@ember/test-helpers';
import a11yAudit from 'ember-a11y-testing/test-support/audit';
import axeConfig from '../axe-config';
import setupMirage from '../helpers/setup-mirage';
import { percySnapshot } from 'ember-percy';

module('Acceptance | crate page', function(hooks) {
    setupApplicationTest(hooks);
    setupMirage(hooks);

    test('is accessible', async function(assert) {
        assert.expect(0);

        this.server.create('crate', 'withVersion', { id: 'nanomsg' });

        await visit('/');
        percySnapshot(assert);

        await a11yAudit(axeConfig);
    });

    test('/crates/:crate is accessible', async function(assert) {
        assert.expect(0);

        this.server.create('crate', { id: 'nanomsg', max_version: '0.6.1' });
        this.server.create('version', { crate: 'nanomsg', num: '0.6.0' });
        this.server.create('version', { crate: 'nanomsg', num: '0.6.1' });

        await visit('/crates/nanomsg');
        percySnapshot(assert);

        await a11yAudit(axeConfig);
    });

    test('/crates/:crate/:version is accessible', async function(assert) {
        assert.expect(0);

        this.server.create('crate', { id: 'nanomsg', max_version: '0.6.1' });
        this.server.create('version', { crate: 'nanomsg', num: '0.6.0' });
        this.server.create('version', { crate: 'nanomsg', num: '0.6.1' });

        await visit('/crates/nanomsg/0.6.0');
        percySnapshot(assert);

        await a11yAudit(axeConfig);
    });

    test('/crates/:crate/owners is accessible', async function(assert) {
        assert.expect(0);

        this.server.loadFixtures();

        await visit('/crates/nanomsg/owners');
        percySnapshot(assert);

        await a11yAudit(axeConfig);
    });

    test('visiting a crate page from the front page', async function(assert) {
        this.server.create('crate', 'withVersion', { id: 'nanomsg' });

        await visit('/');
        await click('[data-test-just-updated] [data-test-crate-link="0"]');

        assert.equal(currentURL(), '/crates/nanomsg');
        assert.equal(document.title, 'nanomsg - Cargo: packages for Rust');
    });

    test('visiting /crates/nanomsg', async function(assert) {
        this.server.create('crate', { id: 'nanomsg', max_version: '0.6.1' });
        this.server.create('version', { crate: 'nanomsg', num: '0.6.0' });
        this.server.create('version', { crate: 'nanomsg', num: '0.6.1' });

        await visit('/crates/nanomsg');

        assert.equal(currentURL(), '/crates/nanomsg');
        assert.equal(currentRouteName(), 'crate.index');
        assert.equal(document.title, 'nanomsg - Cargo: packages for Rust');

        assert.dom('[data-test-heading] [data-test-crate-name]').hasText('nanomsg');
        assert.dom('[data-test-heading] [data-test-crate-version]').hasText('0.6.1');
    });

    test('visiting /crates/nanomsg/', async function(assert) {
        this.server.create('crate', { id: 'nanomsg', max_version: '0.6.1' });
        this.server.create('version', { crate: 'nanomsg', num: '0.6.0' });
        this.server.create('version', { crate: 'nanomsg', num: '0.6.1' });

        await visit('/crates/nanomsg/');

        assert.equal(currentURL(), '/crates/nanomsg/');
        assert.equal(currentRouteName(), 'crate.index');
        assert.equal(document.title, 'nanomsg - Cargo: packages for Rust');

        assert.dom('[data-test-heading] [data-test-crate-name]').hasText('nanomsg');
        assert.dom('[data-test-heading] [data-test-crate-version]').hasText('0.6.1');
    });

    test('visiting /crates/nanomsg/0.6.0', async function(assert) {
        this.server.create('crate', { id: 'nanomsg', max_version: '0.6.1' });
        this.server.create('version', { crate: 'nanomsg', num: '0.6.0' });
        this.server.create('version', { crate: 'nanomsg', num: '0.6.1' });

        await visit('/crates/nanomsg/0.6.0');

        assert.equal(currentURL(), '/crates/nanomsg/0.6.0');
        assert.equal(currentRouteName(), 'crate.version');
        assert.equal(document.title, 'nanomsg - Cargo: packages for Rust');

        assert.dom('[data-test-heading] [data-test-crate-name]').hasText('nanomsg');
        assert.dom('[data-test-heading] [data-test-crate-version]').hasText('0.6.0');
    });

    test('navigating to the all versions page', async function(assert) {
        this.server.loadFixtures();

        await visit('/crates/nanomsg');
        await click('[data-test-all-versions-link]');

        assert.dom('.info').hasText(/All 13\s+versions of nanomsg since\s+December \d+, 2014/);
    });

    test('navigating to the reverse dependencies page', async function(assert) {
        this.server.loadFixtures();

        await visit('/crates/nanomsg');
        await click('[data-test-reverse-deps-link]');

        assert.equal(currentURL(), '/crates/nanomsg/reverse_dependencies');
        assert.dom('a[href="/crates/unicorn-rpc"]').hasText('unicorn-rpc');
    });

    test('navigating to a user page', async function(assert) {
        this.server.loadFixtures();

        await visit('/crates/nanomsg');
        await click('[data-test-owners] [data-test-user-link="blabaere"]');

        assert.equal(currentURL(), '/users/blabaere');
        assert.dom('[data-test-heading] [data-test-username]').hasText('blabaere');
    });

    test('navigating to a team page', async function(assert) {
        this.server.loadFixtures();

        await visit('/crates/nanomsg');
        await click('[data-test-owners] [data-test-team-link="github:org:thehydroimpulse"]');

        assert.equal(currentURL(), '/teams/github:org:thehydroimpulse');
        assert.dom('[data-test-heading] [data-test-team-name]').hasText('thehydroimpulseteam');
    });

    test('crates having user-owners', async function(assert) {
        this.server.loadFixtures();

        await visit('/crates/nanomsg');

        assert
            .dom('[data-test-owners] [data-test-team-link="github:org:thehydroimpulse"] img')
            .hasAttribute('src', 'https://avatars.githubusercontent.com/u/565790?v=3&s=64');

        assert.dom('[data-test-owners] li').exists({ count: 4 });
    });

    test('crates having team-owners', async function(assert) {
        this.server.loadFixtures();

        await visit('/crates/nanomsg');

        assert.dom('[data-test-owners] [data-test-team-link="github:org:thehydroimpulse"]').exists();
        assert.dom('[data-test-owners] li').exists({ count: 4 });
    });

    test('crates license is supplied by version', async function(assert) {
        this.server.loadFixtures();

        await visit('/crates/nanomsg');
        assert.dom('[data-test-license]').hasText('Apache-2.0');

        await click('[data-test-version-link="0.5.0"]');
        assert.dom('[data-test-license]').hasText('MIT/Apache-2.0');
    });

    test('navigating to the owners page when not logged in', async function(assert) {
        this.server.loadFixtures();

        await visit('/crates/nanomsg');

        assert.dom('#crate-owners p a').doesNotExist();
    });

    test('navigating to the owners page when not an owner', async function(assert) {
        this.server.loadFixtures();

        this.owner.register(
            'service:session',
            Service.extend({
                get currentUser() {
                    return { login: 'iain8' };
                },
                loadUser() {},
            }),
        );

        await visit('/crates/nanomsg');

        assert.dom('#crate-owners p a').doesNotExist();
    });

    test('navigating to the owners page', async function(assert) {
        this.server.loadFixtures();

        this.owner.register(
            'service:session',
            Service.extend({
                get currentUser() {
                    return { login: 'thehydroimpulse', id: '2' };
                },
                loadUser() {},
            }),
        );

        await visit('/crates/nanomsg');
        await click('#crate-owners p a');

        assert.dom('#crates-heading h1').hasText('Manage Crate Owners');
    });

    test('listing crate owners', async function(assert) {
        this.server.loadFixtures();

        await visit('/crates/nanomsg/owners');

        assert.dom('.owners .row').exists({ count: 2 });
        assert.dom('a[href="/users/thehydroimpulse"]').exists();
        assert.dom('a[href="/users/blabaere"]').exists();
    });

    test('attempting to add owner without username', async function(assert) {
        this.server.loadFixtures();

        await visit('/crates/nanomsg/owners');
        await click('#add-owner');

        assert.dom('.error').exists();
        assert.dom('.error').hasText('Please enter a username');
        assert.dom('.owners .row').exists({ count: 2 });
    });

    test('attempting to add non-existent owner', async function(assert) {
        this.server.loadFixtures();

        await visit('/crates/nanomsg/owners');
        await fillIn('input[name="username"]', 'spookyghostboo');
        await click('#add-owner');

        assert.dom('.error').exists();
        assert.dom('.error').hasText('Error sending invite');
        assert.dom('.owners .row').exists({ count: 2 });
    });

    test('add a new owner', async function(assert) {
        this.server.loadFixtures();

        await visit('/crates/nanomsg/owners');
        await fillIn('input[name="username"]', 'iain8');
        await click('#add-owner');

        assert.dom('.invited').exists();
        assert.dom('.invited').hasText('An invite has been sent to iain8');
        assert.dom('.owners .row').exists({ count: 2 });
    });

    test('remove a crate owner', async function(assert) {
        this.server.loadFixtures();

        await visit('/crates/nanomsg/owners');
        await click('.owners .row:first-child .remove-owner');

        assert.dom('.removed').exists();
        assert.dom('.owners .row').exists({ count: 1 });
    });
});
