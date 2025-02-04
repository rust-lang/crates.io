import { click, currentRouteName, currentURL, waitFor } from '@ember/test-helpers';
import { module, skip, test } from 'qunit';

import { loadFixtures } from '@crates-io/msw/fixtures.js';
import percySnapshot from '@percy/ember';
import a11yAudit from 'ember-a11y-testing/test-support/audit';
import { getPageTitle } from 'ember-page-title/test-support';
import { http, HttpResponse } from 'msw';

import { setupApplicationTest } from 'crates-io/tests/helpers';

import axeConfig from '../axe-config';
import { visit } from '../helpers/visit-ignoring-abort';

module('Acceptance | crate page', function (hooks) {
  setupApplicationTest(hooks, { msw: true });

  test('visiting a crate page from the front page', async function (assert) {
    let crate = this.db.crate.create({ name: 'nanomsg', newest_version: '0.6.1' });
    this.db.version.create({ crate, num: '0.6.1' });

    await visit('/');
    await click('[data-test-just-updated] [data-test-crate-link="0"]');

    assert.strictEqual(currentURL(), '/crates/nanomsg/0.6.1');
    assert.strictEqual(getPageTitle(), 'nanomsg - crates.io: Rust Package Registry');

    assert.dom('[data-test-heading] [data-test-crate-name]').hasText('nanomsg');
    assert.dom('[data-test-heading] [data-test-crate-version]').hasText('v0.6.1');
  });

  test('visiting /crates/nanomsg', async function (assert) {
    let crate = this.db.crate.create({ name: 'nanomsg' });
    this.db.version.create({ crate, num: '0.6.0' });
    this.db.version.create({ crate, num: '0.6.1', rust_version: '1.69' });

    await visit('/crates/nanomsg');

    assert.strictEqual(currentURL(), '/crates/nanomsg');
    assert.strictEqual(currentRouteName(), 'crate.index');
    assert.strictEqual(getPageTitle(), 'nanomsg - crates.io: Rust Package Registry');

    assert.dom('[data-test-heading] [data-test-crate-name]').hasText('nanomsg');
    assert.dom('[data-test-heading] [data-test-crate-version]').hasText('v0.6.1');
    assert.dom('[data-test-crate-stats-label]').hasText('Stats Overview');

    await percySnapshot(assert);
    await a11yAudit(axeConfig);
  });

  test('visiting /crates/nanomsg/', async function (assert) {
    let crate = this.db.crate.create({ name: 'nanomsg' });
    this.db.version.create({ crate, num: '0.6.0' });
    this.db.version.create({ crate, num: '0.6.1' });

    await visit('/crates/nanomsg/');

    assert.strictEqual(currentURL(), '/crates/nanomsg/');
    assert.strictEqual(currentRouteName(), 'crate.index');
    assert.strictEqual(getPageTitle(), 'nanomsg - crates.io: Rust Package Registry');

    assert.dom('[data-test-heading] [data-test-crate-name]').hasText('nanomsg');
    assert.dom('[data-test-heading] [data-test-crate-version]').hasText('v0.6.1');
    assert.dom('[data-test-crate-stats-label]').hasText('Stats Overview');
  });

  test('visiting /crates/nanomsg/0.6.0', async function (assert) {
    let crate = this.db.crate.create({ name: 'nanomsg' });
    this.db.version.create({ crate, num: '0.6.0' });
    this.db.version.create({ crate, num: '0.6.1' });

    await visit('/crates/nanomsg/0.6.0');

    assert.strictEqual(currentURL(), '/crates/nanomsg/0.6.0');
    assert.strictEqual(currentRouteName(), 'crate.version');
    assert.strictEqual(getPageTitle(), 'nanomsg - crates.io: Rust Package Registry');

    assert.dom('[data-test-heading] [data-test-crate-name]').hasText('nanomsg');
    assert.dom('[data-test-heading] [data-test-crate-version]').hasText('v0.6.0');
    assert.dom('[data-test-crate-stats-label]').hasText('Stats Overview for 0.6.0 (see all)');

    await percySnapshot(assert);
    await a11yAudit(axeConfig);
  });

  test('unknown crate shows an error message', async function (assert) {
    await visit('/crates/nanomsg');
    assert.strictEqual(currentURL(), '/crates/nanomsg');
    assert.dom('[data-test-404-page]').exists();
    assert.dom('[data-test-title]').hasText('nanomsg: Crate not found');
    assert.dom('[data-test-go-back]').exists();
    assert.dom('[data-test-try-again]').doesNotExist();
  });

  test('other crate loading error shows an error message', async function (assert) {
    this.worker.use(http.get('/api/v1/crates/:crate_name', () => HttpResponse.json({}, { status: 500 })));

    await visit('/crates/nanomsg');
    assert.strictEqual(currentURL(), '/crates/nanomsg');
    assert.dom('[data-test-404-page]').exists();
    assert.dom('[data-test-title]').hasText('nanomsg: Failed to load crate data');
    assert.dom('[data-test-go-back]').doesNotExist();
    assert.dom('[data-test-try-again]').exists();
  });

  test('unknown versions fall back to latest version and show an error message', async function (assert) {
    let crate = this.db.crate.create({ name: 'nanomsg' });
    this.db.version.create({ crate, num: '0.6.0' });
    this.db.version.create({ crate, num: '0.6.1' });

    await visit('/crates/nanomsg/0.7.0');

    assert.strictEqual(currentURL(), '/crates/nanomsg/0.7.0');
    assert.dom('[data-test-404-page]').exists();
    assert.dom('[data-test-title]').hasText('nanomsg: Version 0.7.0 not found');
    assert.dom('[data-test-go-back]').exists();
    assert.dom('[data-test-try-again]').doesNotExist();
  });

  test('other versions loading error shows an error message', async function (assert) {
    let crate = this.db.crate.create({ name: 'nanomsg' });
    this.db.version.create({ crate, num: '0.6.0' });
    this.db.version.create({ crate, num: '0.6.1' });

    this.worker.use(http.get('/api/v1/crates/:crate_name/versions', () => HttpResponse.json({}, { status: 500 })));

    await visit('/');
    await click('[data-test-just-updated] [data-test-crate-link="0"]');
    assert.strictEqual(currentURL(), '/crates/nanomsg/0.6.0');
    assert.dom('[data-test-404-page]').exists();
    assert.dom('[data-test-title]').hasText('nanomsg: Failed to load version data');
    assert.dom('[data-test-go-back]').doesNotExist();
    assert.dom('[data-test-try-again]').exists();
  });

  test('works for non-canonical names', async function (assert) {
    let crate = this.db.crate.create({ name: 'foo-bar' });
    this.db.version.create({ crate });

    await visit('/crates/foo_bar');

    assert.strictEqual(currentURL(), '/crates/foo_bar');
    assert.strictEqual(currentRouteName(), 'crate.index');
    assert.strictEqual(getPageTitle(), 'foo-bar - crates.io: Rust Package Registry');

    assert.dom('[data-test-heading] [data-test-crate-name]').hasText('foo-bar');
  });

  test('navigating to the all versions page', async function (assert) {
    loadFixtures(this.db);

    await visit('/crates/nanomsg');
    await click('[data-test-versions-tab] a');

    assert.dom('[data-test-page-description]').hasText(/All 13\s+versions of nanomsg since\s+December \d+th, 2014/);
  });

  test('navigating to the reverse dependencies page', async function (assert) {
    loadFixtures(this.db);

    await visit('/crates/nanomsg');
    await click('[data-test-rev-deps-tab] a');

    assert.strictEqual(currentURL(), '/crates/nanomsg/reverse_dependencies');
    assert.dom('a[href="/crates/unicorn-rpc"]').hasText('unicorn-rpc');
  });

  test('navigating to a user page', async function (assert) {
    loadFixtures(this.db);

    await visit('/crates/nanomsg');
    await click('[data-test-owners] [data-test-owner-link="blabaere"]');

    assert.strictEqual(currentURL(), '/users/blabaere');
    assert.dom('[data-test-heading] [data-test-username]').hasText('blabaere');
  });

  test('navigating to a team page', async function (assert) {
    loadFixtures(this.db);

    await visit('/crates/nanomsg');
    await click('[data-test-owners] [data-test-owner-link="github:org:thehydroimpulse"]');

    assert.strictEqual(currentURL(), '/teams/github:org:thehydroimpulse');
    assert.dom('[data-test-heading] [data-test-team-name]').hasText('thehydroimpulseteam');
  });

  test('crates having user-owners', async function (assert) {
    loadFixtures(this.db);

    await visit('/crates/nanomsg');

    assert
      .dom('[data-test-owners] [data-test-owner-link="github:org:thehydroimpulse"] img')
      .hasAttribute('src', 'https://avatars.githubusercontent.com/u/565790?v=3&s=64');

    assert.dom('[data-test-owners] li').exists({ count: 4 });
  });

  test('crates having team-owners', async function (assert) {
    loadFixtures(this.db);

    await visit('/crates/nanomsg');

    assert.dom('[data-test-owners] [data-test-owner-link="github:org:thehydroimpulse"]').exists();
    assert.dom('[data-test-owners] li').exists({ count: 4 });
  });

  test('crates license is supplied by version', async function (assert) {
    loadFixtures(this.db);

    await visit('/crates/nanomsg');
    assert.dom('[data-test-license]').hasText('Apache-2.0');

    await visit('/crates/nanomsg/0.5.0');
    assert.dom('[data-test-license]').hasText('MIT OR Apache-2.0');
  });

  skip('crates can be yanked by owner', async function (assert) {
    loadFixtures(this.db);

    let user = this.db.user.findFirst({ where: { login: { equals: 'thehydroimpulse' } } });
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
    loadFixtures(this.db);

    await visit('/crates/nanomsg');

    assert.dom('[data-test-settings-tab]').doesNotExist();
  });

  test('navigating to the owners page when not an owner', async function (assert) {
    loadFixtures(this.db);

    let user = this.db.user.findFirst({ where: { login: { equals: 'iain8' } } });
    this.authenticateAs(user);

    await visit('/crates/nanomsg');

    assert.dom('[data-test-settings-tab]').doesNotExist();
  });

  test('navigating to the settings page', async function (assert) {
    loadFixtures(this.db);

    let user = this.db.user.findFirst({ where: { login: { equals: 'thehydroimpulse' } } });
    this.authenticateAs(user);

    await visit('/crates/nanomsg');
    await click('[data-test-settings-tab] a');

    assert.strictEqual(currentURL(), '/crates/nanomsg/settings');
  });
});
