import { click, currentURL } from '@ember/test-helpers';
import { module, test } from 'qunit';

import percySnapshot from '@percy/ember';
import Response from 'ember-cli-mirage/response';

import { setupApplicationTest } from 'cargo/tests/helpers';

import { visit } from '../helpers/visit-ignoring-abort';

module('Acceptance | /me/pending-invites', function (hooks) {
  setupApplicationTest(hooks);

  function prepare(context) {
    let user = context.server.create('user');
    context.authenticateAs(user);

    let inviter = context.server.create('user', { name: 'janed' });
    let inviter2 = context.server.create('user', { name: 'wycats' });
    context.server.get('/api/v1/me/crate_owner_invitations', function () {
      let users = [this.serialize(inviter, 'user').user, this.serialize(inviter2, 'user').user];

      return {
        crate_owner_invitations: [
          {
            invited_by_username: 'janed',
            crate_name: 'nanomsg',
            crate_id: 42,
            created_at: '2016-12-24T12:34:56Z',
            invitee_id: parseInt(user.id, 10),
            inviter_id: parseInt(inviter.id, 10),
          },
          {
            invited_by_username: 'wycats',
            crate_name: 'ember-rs',
            crate_id: 1,
            created_at: '2020-12-31T12:34:56Z',
            invitee_id: parseInt(user.id, 10),
            inviter_id: parseInt(inviter2.id, 10),
          },
        ],
        users,
      };
    });
  }

  test('redirects to / when not logged in', async function (assert) {
    await visit('/me/pending-invites');
    assert.equal(currentURL(), '/');
    assert.dom('[data-test-notification-message]').hasText('Please log in to proceed');
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

    this.server.get('/api/v1/me/crate_owner_invitations', { crate_owner_invitations: [] });

    await visit('/me/pending-invites');
    assert.equal(currentURL(), '/me/pending-invites');
    assert.dom('[data-test-invite]').doesNotExist();
    assert.dom('[data-test-empty-state]').exists();
  });

  test('invites can be declined', async function (assert) {
    assert.expect(9);

    prepare(this);

    this.server.put('/api/v1/me/crate_owner_invitations/:crate', (schema, request) => {
      assert.deepEqual(request.params, { crate: '42' });

      let body = JSON.parse(request.requestBody);
      assert.strictEqual(body.crate_owner_invite.accepted, false);
      assert.strictEqual(body.crate_owner_invite.crate_id, 42);

      return { crate_owner_invitation: { crate_id: 42, accepted: false } };
    });

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
  });

  test('error message is shown if decline request fails', async function (assert) {
    prepare(this);

    this.server.put('/api/v1/me/crate_owner_invitations/:crate', () => new Response(500));

    await visit('/me/pending-invites');
    assert.equal(currentURL(), '/me/pending-invites');

    await click('[data-test-invite="nanomsg"] [data-test-decline-button]');
    assert.dom('[data-test-notification-message="error"]').containsText('Error in declining invite');
    assert.dom('[data-test-accepted-message]').doesNotExist();
    assert.dom('[data-test-declined-message]').doesNotExist();
  });

  test('invites can be accepted', async function (assert) {
    assert.expect(9);

    prepare(this);

    this.server.put('/api/v1/me/crate_owner_invitations/:crate', (schema, request) => {
      assert.deepEqual(request.params, { crate: '42' });

      let body = JSON.parse(request.requestBody);
      assert.strictEqual(body.crate_owner_invite.accepted, true);
      assert.strictEqual(body.crate_owner_invite.crate_id, 42);

      return { crate_owner_invitation: { crate_id: 42, accepted: true } };
    });

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
  });

  test('error message is shown if accept request fails', async function (assert) {
    prepare(this);

    this.server.put('/api/v1/me/crate_owner_invitations/:crate', () => new Response(500));

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
    this.server.put('/api/v1/me/crate_owner_invitations/:crate', payload, 410);

    await visit('/me/pending-invites');
    assert.equal(currentURL(), '/me/pending-invites');

    await click('[data-test-invite="nanomsg"] [data-test-accept-button]');
    assert.dom('[data-test-notification-message="error"]').hasText('Error in accepting invite: ' + errorMessage);
    assert.dom('[data-test-accepted-message]').doesNotExist();
    assert.dom('[data-test-declined-message]').doesNotExist();
  });
});
