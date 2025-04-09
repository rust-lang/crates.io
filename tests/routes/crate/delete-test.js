import { click, currentURL, fillIn, waitFor } from '@ember/test-helpers';
import { module, test } from 'qunit';

import { defer } from 'rsvp';

import percySnapshot from '@percy/ember';
import { http, HttpResponse } from 'msw';

import { setupApplicationTest } from 'crates-io/tests/helpers';

import { visit } from '../../helpers/visit-ignoring-abort';

module('Route: crate.delete', function (hooks) {
  setupApplicationTest(hooks);

  function prepare(context) {
    let user = context.db.user.create();

    let crate = context.db.crate.create({ name: 'foo' });
    context.db.version.create({ crate });
    context.db.crateOwnership.create({ crate, user });

    context.authenticateAs(user);

    return { user };
  }

  test('unauthenticated', async function (assert) {
    let crate = this.db.crate.create({ name: 'foo' });
    this.db.version.create({ crate });

    await visit('/crates/foo/delete');
    assert.strictEqual(currentURL(), '/crates/foo/delete');
    assert.dom('[data-test-title]').hasText('This page requires authentication');
    assert.dom('[data-test-login]').exists();
  });

  test('not an owner', async function (assert) {
    let user1 = this.db.user.create();
    this.authenticateAs(user1);

    let user2 = this.db.user.create();
    let crate = this.db.crate.create({ name: 'foo' });
    this.db.version.create({ crate });
    this.db.crateOwnership.create({ crate, user: user2 });

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

    await fillIn('[data-test-reason]', "I don't need this crate anymore");
    assert.dom('[data-test-delete-button]').isDisabled();
    await click('[data-test-confirmation-checkbox]');
    assert.dom('[data-test-delete-button]').isEnabled();
    await click('[data-test-delete-button]');

    assert.strictEqual(currentURL(), '/');

    let message = 'Crate foo has been successfully deleted.';
    assert.dom('[data-test-notification-message="success"]').hasText(message);

    let crate = this.db.crate.findFirst({ where: { name: { equals: 'foo' } } });
    assert.strictEqual(crate, null);
  });

  test('loading state', async function (assert) {
    prepare(this);

    let deferred = defer();
    this.worker.use(http.delete('/api/v1/crates/foo', () => deferred.promise));

    await visit('/crates/foo/delete');
    await fillIn('[data-test-reason]', "I don't need this crate anymore");
    await click('[data-test-confirmation-checkbox]');
    let clickPromise = click('[data-test-delete-button]');
    await waitFor('[data-test-spinner]');
    assert.dom('[data-test-confirmation-checkbox]').isDisabled();
    assert.dom('[data-test-delete-button]').isDisabled();

    deferred.resolve();
    await clickPromise;

    assert.strictEqual(currentURL(), '/');
  });

  test('error state', async function (assert) {
    prepare(this);

    let payload = { errors: [{ detail: 'only crates without reverse dependencies can be deleted after 72 hours' }] };
    let error = HttpResponse.json(payload, { status: 422 });
    this.worker.use(http.delete('/api/v1/crates/foo', () => error));

    await visit('/crates/foo/delete');
    await fillIn('[data-test-reason]', "I don't need this crate anymore");
    await click('[data-test-confirmation-checkbox]');
    await click('[data-test-delete-button]');
    assert.strictEqual(currentURL(), '/crates/foo/delete');

    let message = 'Failed to delete crate: only crates without reverse dependencies can be deleted after 72 hours';
    assert.dom('[data-test-notification-message="error"]').hasText(message);
  });
});
