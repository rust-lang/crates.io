import { click, currentURL } from '@ember/test-helpers';
import { module, test } from 'qunit';

import percySnapshot from '@percy/ember';
import { http, HttpResponse } from 'msw';

import { setupApplicationTest } from 'crates-io/tests/helpers';

import { visit } from '../helpers/visit-ignoring-abort';

module('Acceptance | /me/pending-invites', function (hooks) {
  setupApplicationTest(hooks, { msw: true });

  function prepare(context) {
    let inviter = context.db.user.create({ name: 'janed' });
    let inviter2 = context.db.user.create({ name: 'wycats' });

    let user = context.db.user.create();

    let nanomsg = context.db.crate.create({ name: 'nanomsg' });
    context.db.version.create({ crate: nanomsg });
    context.db.crateOwnerInvitation.create({
      crate: nanomsg,
      createdAt: '2016-12-24T12:34:56Z',
      invitee: user,
      inviter,
    });

    let ember = context.db.crate.create({ name: 'ember-rs' });
    context.db.version.create({ crate: ember });
    context.db.crateOwnerInvitation.create({
      crate: ember,
      createdAt: '2020-12-31T12:34:56Z',
      invitee: user,
      inviter: inviter2,
    });

    context.authenticateAs(user);

    return { nanomsg, user };
  }

  test('shows "page requires authentication" error when not logged in', async function (assert) {
    await visit('/me/pending-invites');
    assert.strictEqual(currentURL(), '/me/pending-invites');
    assert.dom('[data-test-title]').hasText('This page requires authentication');
    assert.dom('[data-test-login]').exists();
  });

  test('list all pending crate owner invites', async function (assert) {
    prepare(this);

    await visit('/me/pending-invites');
    assert.strictEqual(currentURL(), '/me/pending-invites');
    assert.dom('[data-test-invite]').exists({ count: 2 });
    assert.dom('[data-test-invite="nanomsg"]').exists();
    assert.dom('[data-test-invite="nanomsg"] [data-test-date]').hasText('11 months ago');
    assert.dom('[data-test-invite="nanomsg"] [data-test-accept-button]').exists();
    assert.dom('[data-test-invite="nanomsg"] [data-test-decline-button]').exists();
    assert.dom('[data-test-invite="ember-rs"]').exists();
    assert.dom('[data-test-invite="ember-rs"] [data-test-crate-link]').hasText('ember-rs');
    assert.dom('[data-test-invite="ember-rs"] [data-test-crate-link]').hasAttribute('href', '/crates/ember-rs');
    assert.dom('[data-test-invite="ember-rs"] [data-test-inviter-link]').hasText('wycats');
    assert.dom('[data-test-invite="ember-rs"] [data-test-inviter-link]').hasAttribute('href', '/users/wycats');
    assert.dom('[data-test-invite="ember-rs"] [data-test-date]').hasText('in about 3 years');
    assert.dom('[data-test-invite="ember-rs"] [data-test-accept-button]').exists();
    assert.dom('[data-test-invite="ember-rs"] [data-test-decline-button]').exists();
    assert.dom('[data-test-error-message]').doesNotExist();
    assert.dom('[data-test-accepted-message]').doesNotExist();
    assert.dom('[data-test-declined-message]').doesNotExist();
  });

  test('shows empty list message', async function (assert) {
    prepare(this);

    this.db.crateOwnerInvitation.deleteMany({});

    await visit('/me/pending-invites');
    assert.strictEqual(currentURL(), '/me/pending-invites');
    assert.dom('[data-test-invite]').doesNotExist();
    assert.dom('[data-test-empty-state]').exists();
  });

  test('invites can be declined', async function (assert) {
    let { nanomsg, user } = prepare(this);

    let { crateOwnerInvitation, crateOwnership } = this.db;
    let invites = crateOwnerInvitation.findMany({
      where: {
        crate: { id: { equals: nanomsg.id } },
        invitee: { id: { equals: user.id } },
      },
    });
    assert.strictEqual(invites.length, 1);

    let owners = crateOwnership.findMany({
      where: {
        crate: { id: { equals: nanomsg.id } },
        user: { id: { equals: user.id } },
      },
    });
    assert.strictEqual(owners.length, 0);

    await visit('/me/pending-invites');
    assert.strictEqual(currentURL(), '/me/pending-invites');

    await click('[data-test-invite="nanomsg"] [data-test-decline-button]');
    assert.dom('[data-test-error-message]').doesNotExist();
    assert.dom('[data-test-accepted-message]').doesNotExist();
    assert
      .dom('[data-test-invite="nanomsg"][data-test-declined-message]')
      .hasText('Declined. You have not been added as an owner of crate nanomsg.');
    assert.dom('[data-test-invite="nanomsg"] [data-test-crate-link]').doesNotExist();
    assert.dom('[data-test-invite="nanomsg"] [data-test-inviter-link]').doesNotExist();

    invites = crateOwnerInvitation.findMany({
      where: {
        crate: { id: { equals: nanomsg.id } },
        invitee: { id: { equals: user.id } },
      },
    });
    assert.strictEqual(invites.length, 0);

    owners = crateOwnership.findMany({
      where: {
        crate: { id: { equals: nanomsg.id } },
        user: { id: { equals: user.id } },
      },
    });
    assert.strictEqual(owners.length, 0);
  });

  test('error message is shown if decline request fails', async function (assert) {
    prepare(this);

    let error = HttpResponse.json({}, { status: 500 });
    this.worker.use(http.put('/api/v1/me/crate_owner_invitations/:crate_id', () => error));

    await visit('/me/pending-invites');
    assert.strictEqual(currentURL(), '/me/pending-invites');

    await click('[data-test-invite="nanomsg"] [data-test-decline-button]');
    assert.dom('[data-test-notification-message="error"]').containsText('Error in declining invite');
    assert.dom('[data-test-accepted-message]').doesNotExist();
    assert.dom('[data-test-declined-message]').doesNotExist();
  });

  test('invites can be accepted', async function (assert) {
    let { nanomsg, user } = prepare(this);

    let { crateOwnerInvitation, crateOwnership } = this.db;
    let invites = crateOwnerInvitation.findMany({
      where: {
        crate: { id: { equals: nanomsg.id } },
        invitee: { id: { equals: user.id } },
      },
    });
    assert.strictEqual(invites.length, 1);

    let owners = crateOwnership.findMany({
      where: {
        crate: { id: { equals: nanomsg.id } },
        user: { id: { equals: user.id } },
      },
    });
    assert.strictEqual(owners.length, 0);

    await visit('/me/pending-invites');
    assert.strictEqual(currentURL(), '/me/pending-invites');

    await click('[data-test-invite="nanomsg"] [data-test-accept-button]');
    assert.dom('[data-test-error-message]').doesNotExist();
    assert.dom('[data-test-declined-message]').doesNotExist();
    assert
      .dom('[data-test-invite="nanomsg"][data-test-accepted-message]')
      .hasText("Success! You've been added as an owner of crate nanomsg.");
    assert.dom('[data-test-invite="nanomsg"] [data-test-crate-link]').doesNotExist();
    assert.dom('[data-test-invite="nanomsg"] [data-test-inviter-link]').doesNotExist();

    await percySnapshot(assert);

    invites = crateOwnerInvitation.findMany({
      where: {
        crate: { id: { equals: nanomsg.id } },
        invitee: { id: { equals: user.id } },
      },
    });
    assert.strictEqual(invites.length, 0);

    owners = crateOwnership.findMany({
      where: {
        crate: { id: { equals: nanomsg.id } },
        user: { id: { equals: user.id } },
      },
    });
    assert.strictEqual(owners.length, 1);
  });

  test('error message is shown if accept request fails', async function (assert) {
    prepare(this);

    let error = HttpResponse.json({}, { status: 500 });
    this.worker.use(http.put('/api/v1/me/crate_owner_invitations/:crate_id', () => error));

    await visit('/me/pending-invites');
    assert.strictEqual(currentURL(), '/me/pending-invites');

    await click('[data-test-invite="nanomsg"] [data-test-accept-button]');
    assert.dom('[data-test-notification-message="error"]').hasText('Error in accepting invite');
    assert.dom('[data-test-accepted-message]').doesNotExist();
    assert.dom('[data-test-declined-message]').doesNotExist();
  });

  test('specific error message is shown if accept request fails', async function (assert) {
    prepare(this);

    let errorMessage =
      'The invitation to become an owner of the demo_crate crate expired. Please reach out to an owner of the crate to request a new invitation.';
    let error = HttpResponse.json({ errors: [{ detail: errorMessage }] }, { status: 410 });
    this.worker.use(http.put('/api/v1/me/crate_owner_invitations/:crate_id', () => error));

    await visit('/me/pending-invites');
    assert.strictEqual(currentURL(), '/me/pending-invites');

    await click('[data-test-invite="nanomsg"] [data-test-accept-button]');
    assert.dom('[data-test-notification-message="error"]').hasText('Error in accepting invite: ' + errorMessage);
    assert.dom('[data-test-accepted-message]').doesNotExist();
    assert.dom('[data-test-declined-message]').doesNotExist();
  });
});
