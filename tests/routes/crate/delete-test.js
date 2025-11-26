import { click, currentURL, fillIn, waitFor } from '@ember/test-helpers';
import { module, test } from 'qunit';

import { defer } from 'rsvp';

import percySnapshot from '@percy/ember';
import { http, HttpResponse } from 'msw';

import { setupApplicationTest } from 'crates-io/tests/helpers';

import { visit } from '../../helpers/visit-ignoring-abort';

module('Route: crate.delete', function (hooks) {
  setupApplicationTest(hooks);

  async function prepare(context) {
    let user = await context.db.user.create();

    let crate = await context.db.crate.create({ name: 'foo' });
    await context.db.version.create({ crate });
    await context.db.crateOwnership.create({ crate, user });

    await context.authenticateAs(user);

    return { user };
  }

  test('unauthenticated', async function (assert) {
    let crate = await this.db.crate.create({ name: 'foo' });
    await this.db.version.create({ crate });

    await visit('/crates/foo/delete');
    assert.strictEqual(currentURL(), '/crates/foo/delete');
    assert.dom('[data-test-title]').hasText('This page requires authentication');
    assert.dom('[data-test-login]').exists();
  });

  test('not an owner', async function (assert) {
    let user1 = await this.db.user.create();
    await this.authenticateAs(user1);

    let user2 = await this.db.user.create();
    let crate = await this.db.crate.create({ name: 'foo' });
    await this.db.version.create({ crate });
    await this.db.crateOwnership.create({ crate, user: user2 });

    await visit('/crates/foo/delete');
    assert.strictEqual(currentURL(), '/crates/foo/delete');
    assert.dom('[data-test-title]').hasText('This page is only accessible by crate owners');
    assert.dom('[data-test-go-back]').exists();
  });

  test('happy path', async function (assert) {
    await prepare(this);

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

    let crate = this.db.crate.findFirst(q => q.where({ name: 'foo' }));
    assert.strictEqual(crate, undefined);
  });

  test('loading state', async function (assert) {
    await prepare(this);

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
    await prepare(this);

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
