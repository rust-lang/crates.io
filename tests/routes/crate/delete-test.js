import { click, currentURL, waitFor } from '@ember/test-helpers';
import { module, test } from 'qunit';

import { defer } from 'rsvp';

import percySnapshot from '@percy/ember';
import { Response } from 'miragejs';

import { setupApplicationTest } from 'crates-io/tests/helpers';

import { visit } from '../../helpers/visit-ignoring-abort';

module('Route: crate.delete', function (hooks) {
  setupApplicationTest(hooks);

  function prepare(context) {
    let user = context.server.create('user');

    let crate = context.server.create('crate', { name: 'foo' });
    context.server.create('version', { crate });
    context.server.create('crate-ownership', { crate, user });

    context.authenticateAs(user);

    return { user };
  }

  test('unauthenticated', async function (assert) {
    let crate = this.server.create('crate', { name: 'foo' });
    this.server.create('version', { crate });

    await visit('/crates/foo/delete');
    assert.strictEqual(currentURL(), '/crates/foo/delete');
    assert.dom('[data-test-title]').hasText('This page requires authentication');
    assert.dom('[data-test-login]').exists();
  });

  test('not an owner', async function (assert) {
    let user1 = this.server.create('user');
    this.authenticateAs(user1);

    let user2 = this.server.create('user');
    let crate = this.server.create('crate', { name: 'foo' });
    this.server.create('version', { crate });
    this.server.create('crate-ownership', { crate, user: user2 });

    await visit('/crates/foo/delete');
    assert.strictEqual(currentURL(), '/crates/foo/delete');
    assert.dom('[data-test-title]').hasText('This page is only accessible by crate owners');
    assert.dom('[data-test-go-back]').exists();
  });

  test('happy path', async function (assert) {
    prepare(this);

    await visit('/crates/foo/delete');
    assert.strictEqual(currentURL(), '/crates/foo/delete');
    assert.dom('[data-test-title]').hasText('Delete the foo crate?');
    await percySnapshot(assert);

    assert.dom('[data-test-delete-button]').isDisabled();
    await click('[data-test-confirmation-checkbox]');
    assert.dom('[data-test-delete-button]').isEnabled();
    await click('[data-test-delete-button]');

    assert.strictEqual(currentURL(), '/');

    let message = 'Crate foo has been successfully deleted.';
    assert.dom('[data-test-notification-message="success"]').hasText(message);

    let crate = this.server.schema.crates.findBy({ name: 'foo' });
    assert.strictEqual(crate, null);
  });

  test('loading state', async function (assert) {
    prepare(this);

    let deferred = defer();
    this.server.delete('/api/v1/crates/foo', deferred.promise);

    await visit('/crates/foo/delete');
    await click('[data-test-confirmation-checkbox]');
    let clickPromise = click('[data-test-delete-button]');
    await waitFor('[data-test-spinner]');
    assert.dom('[data-test-confirmation-checkbox]').isDisabled();
    assert.dom('[data-test-delete-button]').isDisabled();

    deferred.resolve(new Response(204));
    await clickPromise;

    assert.strictEqual(currentURL(), '/');
  });

  test('error state', async function (assert) {
    prepare(this);

    let payload = { errors: [{ detail: 'only crates without reverse dependencies can be deleted after 72 hours' }] };
    this.server.delete('/api/v1/crates/foo', payload, 422);

    await visit('/crates/foo/delete');
    await click('[data-test-confirmation-checkbox]');
    await click('[data-test-delete-button]');
    assert.strictEqual(currentURL(), '/crates/foo/delete');

    let message = 'Failed to delete crate: only crates without reverse dependencies can be deleted after 72 hours';
    assert.dom('[data-test-notification-message="error"]').hasText(message);
  });
});
