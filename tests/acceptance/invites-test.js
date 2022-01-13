import { click, currentURL } from '@ember/test-helpers';
import { module, test } from 'qunit';

import percySnapshot from '@percy/ember';
import { Response } from 'miragejs';

import { setupApplicationTest } from 'cargo/tests/helpers';

import { visit } from '../helpers/visit-ignoring-abort';

module('Acceptance | /me/pending-invites', function (hooks) {
  setupApplicationTest(hooks);

  function prepare(context) {
    let inviter = context.server.create('user', { name: 'janed' });
    let inviter2 = context.server.create('user', { name: 'wycats' });

    let user = context.server.create('user');

    let nanomsg = context.server.create('crate', { name: 'nanomsg' });
    context.server.create('version', { crate: nanomsg });
    context.server.create('crate-owner-invitation', {
      crate: nanomsg,
      createdAt: '2016-12-24T12:34:56Z',
      invitee: user,
      inviter,
    });

    let ember = context.server.create('crate', { name: 'ember-rs' });
    context.server.create('version', { crate: ember });
    context.server.create('crate-owner-invitation', {
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
    assert.equal(currentURL(), '/me/pending-invites');
    assert.dom('[data-test-title]').hasText('This page requires authentication');
    assert.dom('[data-test-login]').exists();
  });

  test('list all pending crate owner invites', async function (assert) {
    prepare(this);

    await visit('/me/pending-invites');
    assert.equal(currentURL(), '/me/pending-invites');
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

    this.server.schema.crateOwnerInvitations.all().destroy();

    await visit('/me/pending-invites');
    assert.equal(currentURL(), '/me/pending-invites');
    assert.dom('[data-test-invite]').doesNotExist();
    assert.dom('[data-test-empty-state]').exists();
  });

  test('invites can be declined', async function (assert) {
    let { nanomsg, user } = prepare(this);

    let { crateOwnerInvitations, crateOwnerships } = this.server.schema;
    assert.equal(crateOwnerInvitations.where({ crateId: nanomsg.id, inviteeId: user.id }).length, 1);
    assert.equal(crateOwnerships.where({ crateId: nanomsg.id, userId: user.id }).length, 0);

    await visit('/me/pending-invites');
    assert.equal(currentURL(), '/me/pending-invites');

    await click('[data-test-invite="nanomsg"] [data-test-decline-button]');
    assert.dom('[data-test-error-message]').doesNotExist();
    assert.dom('[data-test-accepted-message]').doesNotExist();
    assert
      .dom('[data-test-invite="nanomsg"][data-test-declined-message]')
      .hasText('Declined. You have not been added as an owner of crate nanomsg.');
    assert.dom('[data-test-invite="nanomsg"] [data-test-crate-link]').doesNotExist();
    assert.dom('[data-test-invite="nanomsg"] [data-test-inviter-link]').doesNotExist();

    assert.equal(crateOwnerInvitations.where({ crateId: nanomsg.id, inviteeId: user.id }).length, 0);
    assert.equal(crateOwnerships.where({ crateId: nanomsg.id, userId: user.id }).length, 0);
  });

  test('error message is shown if decline request fails', async function (assert) {
    prepare(this);

    this.server.put('/api/v1/me/crate_owner_invitations/:crate_id', () => new Response(500));

    await visit('/me/pending-invites');
    assert.equal(currentURL(), '/me/pending-invites');

    await click('[data-test-invite="nanomsg"] [data-test-decline-button]');
    assert.dom('[data-test-notification-message="error"]').containsText('Error in declining invite');
    assert.dom('[data-test-accepted-message]').doesNotExist();
    assert.dom('[data-test-declined-message]').doesNotExist();
  });

  test('invites can be accepted', async function (assert) {
    let { nanomsg, user } = prepare(this);

    let { crateOwnerInvitations, crateOwnerships } = this.server.schema;
    assert.equal(crateOwnerInvitations.where({ crateId: nanomsg.id, inviteeId: user.id }).length, 1);
    assert.equal(crateOwnerships.where({ crateId: nanomsg.id, userId: user.id }).length, 0);

    await visit('/me/pending-invites');
    assert.equal(currentURL(), '/me/pending-invites');

    await click('[data-test-invite="nanomsg"] [data-test-accept-button]');
    assert.dom('[data-test-error-message]').doesNotExist();
    assert.dom('[data-test-declined-message]').doesNotExist();
    assert
      .dom('[data-test-invite="nanomsg"][data-test-accepted-message]')
      .hasText("Success! You've been added as an owner of crate nanomsg.");
    assert.dom('[data-test-invite="nanomsg"] [data-test-crate-link]').doesNotExist();
    assert.dom('[data-test-invite="nanomsg"] [data-test-inviter-link]').doesNotExist();

    await percySnapshot(assert);

    assert.equal(crateOwnerInvitations.where({ crateId: nanomsg.id, inviteeId: user.id }).length, 0);
    assert.equal(crateOwnerships.where({ crateId: nanomsg.id, userId: user.id }).length, 1);
  });

  test('error message is shown if accept request fails', async function (assert) {
    prepare(this);

    this.server.put('/api/v1/me/crate_owner_invitations/:crate_id', () => new Response(500));

    await visit('/me/pending-invites');
    assert.equal(currentURL(), '/me/pending-invites');

    await click('[data-test-invite="nanomsg"] [data-test-accept-button]');
    assert.dom('[data-test-notification-message="error"]').hasText('Error in accepting invite');
    assert.dom('[data-test-accepted-message]').doesNotExist();
    assert.dom('[data-test-declined-message]').doesNotExist();
  });

  test('specific error message is shown if accept request fails', async function (assert) {
    prepare(this);

    let errorMessage =
      'The invitation to become an owner of the demo_crate crate expired. Please reach out to an owner of the crate to request a new invitation.';
    let payload = { errors: [{ detail: errorMessage }] };
    this.server.put('/api/v1/me/crate_owner_invitations/:crate_id', payload, 410);

    await visit('/me/pending-invites');
    assert.equal(currentURL(), '/me/pending-invites');

    await click('[data-test-invite="nanomsg"] [data-test-accept-button]');
    assert.dom('[data-test-notification-message="error"]').hasText('Error in accepting invite: ' + errorMessage);
    assert.dom('[data-test-accepted-message]').doesNotExist();
    assert.dom('[data-test-declined-message]').doesNotExist();
  });
});
