import { click, fillIn, currentURL, currentRouteName, visit } from '@ember/test-helpers';
import { setupApplicationTest } from 'ember-qunit';
import { module, test } from 'qunit';

import a11yAudit from 'ember-a11y-testing/test-support/audit';
import { percySnapshot } from 'ember-percy';

import axeConfig from '../axe-config';
import { title } from '../helpers/dom';
import setupMirage from '../helpers/setup-mirage';

module('Acceptance | crate page', function (hooks) {
  setupApplicationTest(hooks);
  setupMirage(hooks);

  test('/crates/:crate is accessible', async function (assert) {
    assert.expect(0);

    this.server.create('crate', { name: 'nanomsg' });
    this.server.create('version', { crateId: 'nanomsg', num: '0.6.0' });
    this.server.create('version', { crateId: 'nanomsg', num: '0.6.1' });

    await visit('/crates/nanomsg');
    percySnapshot(assert);

    await a11yAudit(axeConfig);
  });

  test('/crates/:crate/:version is accessible', async function (assert) {
    assert.expect(0);

    this.server.create('crate', { name: 'nanomsg' });
    this.server.create('version', { crateId: 'nanomsg', num: '0.6.0' });
    this.server.create('version', { crateId: 'nanomsg', num: '0.6.1' });

    await visit('/crates/nanomsg/0.6.0');
    percySnapshot(assert);

    await a11yAudit(axeConfig);
  });

  test('/crates/:crate/owners is accessible', async function (assert) {
    assert.expect(0);

    this.server.loadFixtures();

    await visit('/crates/nanomsg/owners');
    percySnapshot(assert);

    await a11yAudit(axeConfig);
  });

  test('visiting a crate page from the front page', async function (assert) {
    this.server.create('crate', { name: 'nanomsg', newest_version: '0.6.1' });
    this.server.create('version', { crateId: 'nanomsg', num: '0.6.1' });

    await visit('/');
    await click('[data-test-just-updated] [data-test-crate-link="0"]');

    assert.equal(currentURL(), '/crates/nanomsg');
    assert.equal(title(), 'nanomsg - crates.io: Rust Package Registry');

    assert.dom('[data-test-heading] [data-test-crate-name]').hasText('nanomsg');
    assert.dom('[data-test-heading] [data-test-crate-version]').hasText('0.6.1');
  });

  test('visiting /crates/nanomsg', async function (assert) {
    this.server.create('crate', { name: 'nanomsg' });
    this.server.create('version', { crateId: 'nanomsg', num: '0.6.0' });
    this.server.create('version', { crateId: 'nanomsg', num: '0.6.1' });

    await visit('/crates/nanomsg');

    assert.equal(currentURL(), '/crates/nanomsg');
    assert.equal(currentRouteName(), 'crate.index');
    assert.equal(title(), 'nanomsg - crates.io: Rust Package Registry');

    assert.dom('[data-test-heading] [data-test-crate-name]').hasText('nanomsg');
    assert.dom('[data-test-heading] [data-test-crate-version]').hasText('0.6.1');
    assert.dom('[data-test-crate-stats-label]').hasText('Stats Overview');
  });

  test('visiting /crates/nanomsg/', async function (assert) {
    this.server.create('crate', { name: 'nanomsg' });
    this.server.create('version', { crateId: 'nanomsg', num: '0.6.0' });
    this.server.create('version', { crateId: 'nanomsg', num: '0.6.1' });

    await visit('/crates/nanomsg/');

    assert.equal(currentURL(), '/crates/nanomsg/');
    assert.equal(currentRouteName(), 'crate.index');
    assert.equal(title(), 'nanomsg - crates.io: Rust Package Registry');

    assert.dom('[data-test-heading] [data-test-crate-name]').hasText('nanomsg');
    assert.dom('[data-test-heading] [data-test-crate-version]').hasText('0.6.1');
    assert.dom('[data-test-crate-stats-label]').hasText('Stats Overview');
  });

  test('visiting /crates/nanomsg/0.6.0', async function (assert) {
    this.server.create('crate', { name: 'nanomsg' });
    this.server.create('version', { crateId: 'nanomsg', num: '0.6.0' });
    this.server.create('version', { crateId: 'nanomsg', num: '0.6.1' });

    await visit('/crates/nanomsg/0.6.0');

    assert.equal(currentURL(), '/crates/nanomsg/0.6.0');
    assert.equal(currentRouteName(), 'crate.version');
    assert.equal(title(), 'nanomsg - crates.io: Rust Package Registry');

    assert.dom('[data-test-heading] [data-test-crate-name]').hasText('nanomsg');
    assert.dom('[data-test-heading] [data-test-crate-version]').hasText('0.6.0');
    assert.dom('[data-test-crate-stats-label]').hasText('Stats Overview for 0.6.0 (see all)');
  });

  test('navigating to the all versions page', async function (assert) {
    this.server.loadFixtures();

    await visit('/crates/nanomsg');
    await click('[data-test-all-versions-link]');

    assert.dom('[data-test-page-description]').hasText(/All 13\s+versions of nanomsg since\s+December \d+, 2014/);
  });

  test('navigating to the reverse dependencies page', async function (assert) {
    this.server.loadFixtures();

    await visit('/crates/nanomsg');
    await click('[data-test-reverse-deps-link]');

    assert.equal(currentURL(), '/crates/nanomsg/reverse_dependencies');
    assert.dom('a[href="/crates/unicorn-rpc"]').hasText('unicorn-rpc');
  });

  test('navigating to a user page', async function (assert) {
    this.server.loadFixtures();

    await visit('/crates/nanomsg');
    await click('[data-test-owners] [data-test-user-link="blabaere"]');

    assert.equal(currentURL(), '/users/blabaere');
    assert.dom('[data-test-heading] [data-test-username]').hasText('blabaere');
  });

  test('navigating to a team page', async function (assert) {
    this.server.loadFixtures();

    await visit('/crates/nanomsg');
    await click('[data-test-owners] [data-test-team-link="github:org:thehydroimpulse"]');

    assert.equal(currentURL(), '/teams/github:org:thehydroimpulse');
    assert.dom('[data-test-heading] [data-test-team-name]').hasText('thehydroimpulseteam');
  });

  test('crates having normal dependencies', async function (assert) {
    this.server.loadFixtures();

    await visit('crates/nanomsg');

    assert.dom('[data-test-dependencies] li').exists({ count: 2 });
  });

  test('crates having build dependencies', async function (assert) {
    this.server.loadFixtures();

    await visit('crates/nanomsg');

    assert.dom('[data-test-build-dependencies] li').exists({ count: 1 });
  });

  test('crates having dev dependencies', async function (assert) {
    this.server.loadFixtures();

    await visit('crates/nanomsg');

    assert.dom('[data-test-dev-dependencies] li').exists({ count: 1 });
  });

  test('crates having user-owners', async function (assert) {
    this.server.loadFixtures();

    await visit('/crates/nanomsg');

    assert
      .dom('[data-test-owners] [data-test-team-link="github:org:thehydroimpulse"] img')
      .hasAttribute('src', 'https://avatars.githubusercontent.com/u/565790?v=3&s=64');

    assert.dom('[data-test-owners] li').exists({ count: 4 });
  });

  test('crates having team-owners', async function (assert) {
    this.server.loadFixtures();

    await visit('/crates/nanomsg');

    assert.dom('[data-test-owners] [data-test-team-link="github:org:thehydroimpulse"]').exists();
    assert.dom('[data-test-owners] li').exists({ count: 4 });
  });

  test('crates license is supplied by version', async function (assert) {
    this.server.loadFixtures();

    await visit('/crates/nanomsg');
    assert.dom('[data-test-license]').hasText('Apache-2.0');

    await click('[data-test-version-link="0.5.0"]');
    assert.dom('[data-test-license]').hasText('MIT/Apache-2.0');
  });

  test('navigating to the owners page when not logged in', async function (assert) {
    this.server.loadFixtures();

    await visit('/crates/nanomsg');

    assert.dom('[data-test-manage-owners-link]').doesNotExist();
  });

  test('navigating to the owners page when not an owner', async function (assert) {
    this.server.loadFixtures();

    let user = this.server.schema.users.findBy({ login: 'iain8' });
    this.authenticateAs(user);

    await visit('/crates/nanomsg');

    assert.dom('[data-test-manage-owners-link]').doesNotExist();
  });

  test('navigating to the owners page', async function (assert) {
    this.server.loadFixtures();

    let user = this.server.schema.users.findBy({ login: 'thehydroimpulse' });
    this.authenticateAs(user);

    await visit('/crates/nanomsg');
    await click('[data-test-manage-owners-link]');

    assert.equal(currentURL(), '/crates/nanomsg/owners');
  });

  test('listing crate owners', async function (assert) {
    this.server.loadFixtures();

    await visit('/crates/nanomsg/owners');

    assert.dom('[data-test-owners] [data-test-owner-team]').exists({ count: 2 });
    assert.dom('[data-test-owners] [data-test-owner-user]').exists({ count: 2 });
    assert.dom('a[href="/teams/github:org:thehydroimpulse"]').exists();
    assert.dom('a[href="/teams/github:org:blabaere"]').exists();
    assert.dom('a[href="/users/thehydroimpulse"]').exists();
    assert.dom('a[href="/users/blabaere"]').exists();
  });

  test('attempting to add owner without username', async function (assert) {
    this.server.loadFixtures();

    await visit('/crates/nanomsg/owners');
    await click('[data-test-save-button]');

    assert.dom('[data-test-error-message]').hasText('Please enter a username');
    assert.dom('[data-test-owners] [data-test-owner-team]').exists({ count: 2 });
    assert.dom('[data-test-owners] [data-test-owner-user]').exists({ count: 2 });
  });

  test('attempting to add non-existent owner', async function (assert) {
    this.server.loadFixtures();

    await visit('/crates/nanomsg/owners');
    await fillIn('input[name="username"]', 'spookyghostboo');
    await click('[data-test-save-button]');

    assert
      .dom('[data-test-error-message]')
      .hasText('Error sending invite: could not find user with login `spookyghostboo`');
    assert.dom('[data-test-owners] [data-test-owner-team]').exists({ count: 2 });
    assert.dom('[data-test-owners] [data-test-owner-user]').exists({ count: 2 });
  });

  test('add a new owner', async function (assert) {
    this.server.loadFixtures();

    await visit('/crates/nanomsg/owners');
    await fillIn('input[name="username"]', 'iain8');
    await click('[data-test-save-button]');

    assert.dom('[data-test-invited-message]').hasText('An invite has been sent to iain8');
    assert.dom('[data-test-owners] [data-test-owner-team]').exists({ count: 2 });
    assert.dom('[data-test-owners] [data-test-owner-user]').exists({ count: 2 });
  });

  test('remove a crate owner when owner is a user', async function (assert) {
    this.server.loadFixtures();

    await visit('/crates/nanomsg/owners');
    await click('[data-test-owner-user="thehydroimpulse"] [data-test-remove-owner-button]');

    assert.dom('[data-test-removed-message]').hasText('User thehydroimpulse removed as crate owner');
    assert.dom('[data-test-owner-user]').exists({ count: 1 });
  });

  test('remove a crate owner when owner is a team', async function (assert) {
    this.server.loadFixtures();

    await visit('/crates/nanomsg/owners');
    await click('[data-test-owner-team="github:org:thehydroimpulse"] [data-test-remove-owner-button]');

    assert.dom('[data-test-removed-message]').hasText('Team org/thehydroimpulseteam removed as crate owner');
    assert.dom('[data-test-owner-team]').exists({ count: 1 });
  });
});
