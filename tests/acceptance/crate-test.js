import { click, currentRouteName, currentURL, waitFor } from '@ember/test-helpers';
import { module, skip, test } from 'qunit';

import percySnapshot from '@percy/ember';
import a11yAudit from 'ember-a11y-testing/test-support/audit';
import { getPageTitle } from 'ember-page-title/test-support';

import { setupApplicationTest } from 'cargo/tests/helpers';

import axeConfig from '../axe-config';
import { visit } from '../helpers/visit-ignoring-abort';

module('Acceptance | crate page', function (hooks) {
  setupApplicationTest(hooks);

  test('visiting a crate page from the front page', async function (assert) {
    let crate = this.server.create('crate', { name: 'nanomsg', newest_version: '0.6.1' });
    this.server.create('version', { crate, num: '0.6.1' });

    await visit('/');
    await click('[data-test-just-updated] [data-test-crate-link="0"]');

    assert.equal(currentURL(), '/crates/nanomsg');
    assert.equal(getPageTitle(), 'nanomsg - crates.io: Rust Package Registry');

    assert.dom('[data-test-heading] [data-test-crate-name]').hasText('nanomsg');
    assert.dom('[data-test-heading] [data-test-crate-version]').hasText('0.6.1');
  });

  test('visiting /crates/nanomsg', async function (assert) {
    let crate = this.server.create('crate', { name: 'nanomsg' });
    this.server.create('version', { crate, num: '0.6.0' });
    this.server.create('version', { crate, num: '0.6.1' });

    await visit('/crates/nanomsg');

    assert.equal(currentURL(), '/crates/nanomsg');
    assert.equal(currentRouteName(), 'crate.index');
    assert.equal(getPageTitle(), 'nanomsg - crates.io: Rust Package Registry');

    assert.dom('[data-test-heading] [data-test-crate-name]').hasText('nanomsg');
    assert.dom('[data-test-heading] [data-test-crate-version]').hasText('0.6.1');
    assert.dom('[data-test-crate-stats-label]').hasText('Stats Overview');

    await percySnapshot(assert);
    await a11yAudit(axeConfig);
  });

  test('visiting /crates/nanomsg/', async function (assert) {
    let crate = this.server.create('crate', { name: 'nanomsg' });
    this.server.create('version', { crate, num: '0.6.0' });
    this.server.create('version', { crate, num: '0.6.1' });

    await visit('/crates/nanomsg/');

    assert.equal(currentURL(), '/crates/nanomsg/');
    assert.equal(currentRouteName(), 'crate.index');
    assert.equal(getPageTitle(), 'nanomsg - crates.io: Rust Package Registry');

    assert.dom('[data-test-heading] [data-test-crate-name]').hasText('nanomsg');
    assert.dom('[data-test-heading] [data-test-crate-version]').hasText('0.6.1');
    assert.dom('[data-test-crate-stats-label]').hasText('Stats Overview');
  });

  test('visiting /crates/nanomsg/0.6.0', async function (assert) {
    let crate = this.server.create('crate', { name: 'nanomsg' });
    this.server.create('version', { crate, num: '0.6.0' });
    this.server.create('version', { crate, num: '0.6.1' });

    await visit('/crates/nanomsg/0.6.0');

    assert.equal(currentURL(), '/crates/nanomsg/0.6.0');
    assert.equal(currentRouteName(), 'crate.version');
    assert.equal(getPageTitle(), 'nanomsg - crates.io: Rust Package Registry');

    assert.dom('[data-test-heading] [data-test-crate-name]').hasText('nanomsg');
    assert.dom('[data-test-heading] [data-test-crate-version]').hasText('0.6.0');
    assert.dom('[data-test-crate-stats-label]').hasText('Stats Overview for 0.6.0 (see all)');

    await percySnapshot(assert);
    await a11yAudit(axeConfig);
  });

  test('unknown crate shows an error message', async function (assert) {
    await visit('/crates/nanomsg');
    assert.equal(currentURL(), '/crates/nanomsg');
    assert.dom('[data-test-404-page]').exists();
    assert.dom('[data-test-title]').hasText('Crate not found');
    assert.dom('[data-test-go-back]').exists();
    assert.dom('[data-test-try-again]').doesNotExist();
  });

  test('other crate loading error shows an error message', async function (assert) {
    this.server.get('/api/v1/crates/:crate_name', {}, 500);

    await visit('/crates/nanomsg');
    assert.equal(currentURL(), '/crates/nanomsg');
    assert.dom('[data-test-404-page]').exists();
    assert.dom('[data-test-title]').hasText('Crate failed to load');
    assert.dom('[data-test-go-back]').doesNotExist();
    assert.dom('[data-test-try-again]').exists();
  });

  test('unknown versions fall back to latest version and show an error message', async function (assert) {
    let crate = this.server.create('crate', { name: 'nanomsg' });
    this.server.create('version', { crate, num: '0.6.0' });
    this.server.create('version', { crate, num: '0.6.1' });

    await visit('/crates/nanomsg/0.7.0');

    assert.equal(currentURL(), '/crates/nanomsg/0.7.0');
    assert.dom('[data-test-404-page]').exists();
    assert.dom('[data-test-title]').hasText('Version not found');
    assert.dom('[data-test-go-back]').exists();
    assert.dom('[data-test-try-again]').doesNotExist();
  });

  test('other versions loading error shows an error message', async function (assert) {
    let crate = this.server.create('crate', { name: 'nanomsg' });
    this.server.create('version', { crate, num: '0.6.0' });
    this.server.create('version', { crate, num: '0.6.1' });

    this.server.get('/api/v1/crates/:crate_name/versions', {}, 500);

    await visit('/');
    await click('[data-test-just-updated] [data-test-crate-link="0"]');
    assert.equal(currentURL(), '/crates/nanomsg');
    assert.dom('[data-test-404-page]').exists();
    assert.dom('[data-test-title]').hasText('Crate failed to load');
    assert.dom('[data-test-go-back]').doesNotExist();
    assert.dom('[data-test-try-again]').exists();
  });

  test('navigating to the all versions page', async function (assert) {
    this.server.loadFixtures();

    await visit('/crates/nanomsg');
    await click('[data-test-versions-tab] a');

    assert.dom('[data-test-page-description]').hasText(/All 13\s+versions of nanomsg since\s+December \d+th, 2014/);
  });

  test('navigating to the reverse dependencies page', async function (assert) {
    this.server.loadFixtures();

    await visit('/crates/nanomsg');
    await click('[data-test-rev-deps-tab] a');

    assert.equal(currentURL(), '/crates/nanomsg/reverse_dependencies');
    assert.dom('a[href="/crates/unicorn-rpc"]').hasText('unicorn-rpc');
  });

  test('navigating to a user page', async function (assert) {
    this.server.loadFixtures();

    await visit('/crates/nanomsg');
    await click('[data-test-owners] [data-test-owner-link="blabaere"]');

    assert.equal(currentURL(), '/users/blabaere');
    assert.dom('[data-test-heading] [data-test-username]').hasText('blabaere');
  });

  test('navigating to a team page', async function (assert) {
    this.server.loadFixtures();

    await visit('/crates/nanomsg');
    await click('[data-test-owners] [data-test-owner-link="github:org:thehydroimpulse"]');

    assert.equal(currentURL(), '/teams/github:org:thehydroimpulse');
    assert.dom('[data-test-heading] [data-test-team-name]').hasText('thehydroimpulseteam');
  });

  test('crates having user-owners', async function (assert) {
    this.server.loadFixtures();

    await visit('/crates/nanomsg');

    assert
      .dom('[data-test-owners] [data-test-owner-link="github:org:thehydroimpulse"] img')
      .hasAttribute('src', 'https://avatars.githubusercontent.com/u/565790?v=3&s=64');

    assert.dom('[data-test-owners] li').exists({ count: 4 });
  });

  test('crates having team-owners', async function (assert) {
    this.server.loadFixtures();

    await visit('/crates/nanomsg');

    assert.dom('[data-test-owners] [data-test-owner-link="github:org:thehydroimpulse"]').exists();
    assert.dom('[data-test-owners] li').exists({ count: 4 });
  });

  test('crates license is supplied by version', async function (assert) {
    this.server.loadFixtures();

    await visit('/crates/nanomsg');
    assert.dom('[data-test-license]').hasText('Apache-2.0');

    await visit('/crates/nanomsg/0.5.0');
    assert.dom('[data-test-license]').hasText('MIT OR Apache-2.0');
  });

  skip('crates can be yanked by owner', async function (assert) {
    this.server.loadFixtures();

    let user = this.server.schema.users.findBy({ login: 'thehydroimpulse' });
    this.authenticateAs(user);

    await visit('/crates/nanomsg/0.5.0');
    await click('[data-test-version-yank-button="0.5.0"]');
    assert.dom('[data-test-version-yank-button="0.5.0"]').hasText('Yanking...');
    assert.dom('[data-test-version-yank-button="0.5.0"]').isDisabled();

    await waitFor('[data-test-version-unyank-button="0.5.0"]');
    await click('[data-test-version-unyank-button="0.5.0"]');
    assert.dom('[data-test-version-unyank-button="0.5.0"]').hasText('Unyanking...');
    assert.dom('[data-test-version-unyank-button="0.5.0"]').isDisabled();

    await waitFor('[data-test-version-yank-button="0.5.0"]');
  });

  test('navigating to the owners page when not logged in', async function (assert) {
    this.server.loadFixtures();

    await visit('/crates/nanomsg');

    assert.dom('[data-test-settings-tab]').doesNotExist();
  });

  test('navigating to the owners page when not an owner', async function (assert) {
    this.server.loadFixtures();

    let user = this.server.schema.users.findBy({ login: 'iain8' });
    this.authenticateAs(user);

    await visit('/crates/nanomsg');

    assert.dom('[data-test-settings-tab]').doesNotExist();
  });

  test('navigating to the settings page', async function (assert) {
    this.server.loadFixtures();

    let user = this.server.schema.users.findBy({ login: 'thehydroimpulse' });
    this.authenticateAs(user);

    await visit('/crates/nanomsg');
    await click('[data-test-settings-tab] a');

    assert.equal(currentURL(), '/crates/nanomsg/settings');
  });
});
