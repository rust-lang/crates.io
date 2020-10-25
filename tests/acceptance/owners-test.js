import { click, fillIn, visit } from '@ember/test-helpers';
import { setupApplicationTest } from 'ember-qunit';
import { module, test } from 'qunit';

import percySnapshot from '@percy/ember';
import a11yAudit from 'ember-a11y-testing/test-support/audit';

import axeConfig from '../axe-config';
import setupMirage from '../helpers/setup-mirage';

module('Acceptance | /crates/:name/owners', function (hooks) {
  setupApplicationTest(hooks);
  setupMirage(hooks);

  test('listing crate owners', async function (assert) {
    this.server.loadFixtures();

    await visit('/crates/nanomsg/owners');

    assert.dom('[data-test-owners] [data-test-owner-team]').exists({ count: 2 });
    assert.dom('[data-test-owners] [data-test-owner-user]').exists({ count: 2 });
    assert.dom('a[href="/teams/github:org:thehydroimpulse"]').exists();
    assert.dom('a[href="/teams/github:org:blabaere"]').exists();
    assert.dom('a[href="/users/thehydroimpulse"]').exists();
    assert.dom('a[href="/users/blabaere"]').exists();

    await percySnapshot(assert);
    await a11yAudit(axeConfig);
  });

  test('attempting to add owner without username', async function (assert) {
    this.server.loadFixtures();

    await visit('/crates/nanomsg/owners');
    await fillIn('input[name="username"]', '');
    assert.dom('[data-test-save-button]').isDisabled();
  });

  test('attempting to add non-existent owner', async function (assert) {
    this.server.loadFixtures();

    await visit('/crates/nanomsg/owners');
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

    await visit('/crates/nanomsg/owners');
    await fillIn('input[name="username"]', 'iain8');
    await click('[data-test-save-button]');

    assert.dom('[data-test-notification-message="success"]').hasText('An invite has been sent to iain8');
    assert.dom('[data-test-owners] [data-test-owner-team]').exists({ count: 2 });
    assert.dom('[data-test-owners] [data-test-owner-user]').exists({ count: 2 });
  });

  test('remove a crate owner when owner is a user', async function (assert) {
    this.server.loadFixtures();

    await visit('/crates/nanomsg/owners');
    await click('[data-test-owner-user="thehydroimpulse"] [data-test-remove-owner-button]');

    assert.dom('[data-test-notification-message="success"]').hasText('User thehydroimpulse removed as crate owner');
    assert.dom('[data-test-owner-user]').exists({ count: 1 });
  });

  test('remove a crate owner when owner is a team', async function (assert) {
    this.server.loadFixtures();

    await visit('/crates/nanomsg/owners');
    await click('[data-test-owner-team="github:org:thehydroimpulse"] [data-test-remove-owner-button]');

    assert
      .dom('[data-test-notification-message="success"]')
      .hasText('Team org/thehydroimpulseteam removed as crate owner');
    assert.dom('[data-test-owner-team]').exists({ count: 1 });
  });
});
