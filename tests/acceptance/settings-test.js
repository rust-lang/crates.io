import { click, currentURL, fillIn, visit } from '@ember/test-helpers';
import { module, test } from 'qunit';

import percySnapshot from '@percy/ember';
import a11yAudit from 'ember-a11y-testing/test-support/audit';

import { setupApplicationTest } from 'cargo/tests/helpers';

import axeConfig from '../axe-config';

module('Acceptance | /crates/:name/settings', function (hooks) {
  setupApplicationTest(hooks);

  test('listing crate owners', async function (assert) {
    this.server.loadFixtures();

    await visit('/crates/nanomsg/settings');
    assert.equal(currentURL(), '/crates/nanomsg/settings');

    assert.dom('[data-test-owners] [data-test-owner-team]').exists({ count: 2 });
    assert.dom('[data-test-owners] [data-test-owner-user]').exists({ count: 2 });
    assert.dom('a[href="/teams/github:org:thehydroimpulse"]').exists();
    assert.dom('a[href="/teams/github:org:blabaere"]').exists();
    assert.dom('a[href="/users/thehydroimpulse"]').exists();
    assert.dom('a[href="/users/blabaere"]').exists();

    await percySnapshot(assert);
    await a11yAudit(axeConfig);
  });

  test('/crates/:name/owners redirects to /crates/:name/settings', async function (assert) {
    this.server.loadFixtures();

    await visit('/crates/nanomsg/owners');
    assert.equal(currentURL(), '/crates/nanomsg/settings');
  });

  test('attempting to add owner without username', async function (assert) {
    this.server.loadFixtures();

    await visit('/crates/nanomsg/settings');
    await fillIn('input[name="username"]', '');
    assert.dom('[data-test-save-button]').isDisabled();
  });

  test('attempting to add non-existent owner', async function (assert) {
    this.server.loadFixtures();

    await visit('/crates/nanomsg/settings');
    await fillIn('input[name="username"]', 'spookyghostboo');
    await click('[data-test-save-button]');

    assert
      .dom('[data-test-notification-message="error"]')
      .hasText('Error sending invite: could not find user with login `spookyghostboo`');
    assert.dom('[data-test-owners] [data-test-owner-team]').exists({ count: 2 });
    assert.dom('[data-test-owners] [data-test-owner-user]').exists({ count: 2 });
  });

  test('add a new owner', async function (assert) {
    this.server.loadFixtures();

    await visit('/crates/nanomsg/settings');
    await fillIn('input[name="username"]', 'iain8');
    await click('[data-test-save-button]');

    assert.dom('[data-test-notification-message="success"]').hasText('An invite has been sent to iain8');
    assert.dom('[data-test-owners] [data-test-owner-team]').exists({ count: 2 });
    assert.dom('[data-test-owners] [data-test-owner-user]').exists({ count: 2 });
  });

  test('remove a crate owner when owner is a user', async function (assert) {
    this.server.loadFixtures();

    await visit('/crates/nanomsg/settings');
    await click('[data-test-owner-user="thehydroimpulse"] [data-test-remove-owner-button]');

    assert.dom('[data-test-notification-message="success"]').hasText('User thehydroimpulse removed as crate owner');
    assert.dom('[data-test-owner-user]').exists({ count: 1 });
  });

  test('remove a user crate owner (error behavior)', async function (assert) {
    let user = this.server.create('user');
    let user2 = this.server.create('user');

    let crate = this.server.create('crate', { name: 'nanomsg' });
    this.server.create('version', { crate, num: '1.0.0' });
    this.server.create('crate-ownership', { crate, user });
    this.server.create('crate-ownership', { crate, user: user2 });

    // we are intentionally returning a 200 response here, because is what
    // the real backend also returns due to legacy reasons
    this.server.delete('/api/v1/crates/nanomsg/owners', { errors: [{ detail: 'nope' }] });

    this.authenticateAs(user);

    await visit(`/crates/${crate.name}/settings`);
    await click(`[data-test-owner-user="${user2.login}"] [data-test-remove-owner-button]`);

    assert
      .dom('[data-test-notification-message="error"]')
      .hasText('Failed to remove the user user-2 as crate owner: nope');
    assert.dom('[data-test-owner-user]').exists({ count: 2 });
  });

  test('remove a crate owner when owner is a team', async function (assert) {
    this.server.loadFixtures();

    await visit('/crates/nanomsg/settings');
    await click('[data-test-owner-team="github:org:thehydroimpulse"] [data-test-remove-owner-button]');

    assert
      .dom('[data-test-notification-message="success"]')
      .hasText('Team org/thehydroimpulseteam removed as crate owner');
    assert.dom('[data-test-owner-team]').exists({ count: 1 });
  });

  test('remove a team crate owner (error behavior)', async function (assert) {
    let user = this.server.create('user');
    let team = this.server.create('team');

    let crate = this.server.create('crate', { name: 'nanomsg' });
    this.server.create('version', { crate, num: '1.0.0' });
    this.server.create('crate-ownership', { crate, user });
    this.server.create('crate-ownership', { crate, team });

    // we are intentionally returning a 200 response here, because is what
    // the real backend also returns due to legacy reasons
    this.server.delete('/api/v1/crates/nanomsg/owners', { errors: [{ detail: 'nope' }] });

    this.authenticateAs(user);

    await visit(`/crates/${crate.name}/settings`);
    await click(`[data-test-owner-team="${team.login}"] [data-test-remove-owner-button]`);

    assert
      .dom('[data-test-notification-message="error"]')
      .hasText('Failed to remove the team rust-lang/team-1 as crate owner: nope');
    assert.dom('[data-test-owner-team]').exists({ count: 1 });
    assert.dom('[data-test-owner-user]').exists({ count: 1 });
  });
});
