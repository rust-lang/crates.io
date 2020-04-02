import { module, test } from 'qunit';
import { setupApplicationTest } from 'ember-qunit';
import { currentURL, click } from '@ember/test-helpers';
import { percySnapshot } from 'ember-percy';
import Response from 'ember-cli-mirage/response';

import setupMirage from '../helpers/setup-mirage';
import { visit } from '../helpers/visit-ignoring-abort';

module('Acceptance | /me/pending-invites', function (hooks) {
  setupApplicationTest(hooks);
  setupMirage(hooks);

  function prepare(context) {
    let user = context.server.create('user');
    context.authenticateAs(user);

    context.server.get('/api/v1/me/crate_owner_invitations', {
      crate_owner_invitations: [
        {
          invited_by_username: 'janed',
          crate_name: 'nanomsg',
          crate_id: 42,
          created_at: '2016-12-24T12:34:56Z',
        },
        {
          invited_by_username: 'wycats',
          crate_name: 'ember-rs',
          crate_id: 1,
          created_at: '2020-12-31T12:34:56Z',
        },
      ],
    });
  }

  test('redirects to / when not logged in', async function (assert) {
    await visit('/me/pending-invites');
    assert.equal(currentURL(), '/');
    assert.dom('[data-test-flash-message]').hasText('Please log in to proceed');
  });

  test('list all pending crate owner invites', async function (assert) {
    prepare(this);

    await visit('/me/pending-invites');
    assert.equal(currentURL(), '/me/pending-invites');
    assert.dom('[data-test-invite]').exists({ count: 2 });
    assert.dom('[data-test-invite="nanomsg"]').exists();
    assert.dom('[data-test-invite="nanomsg"] [data-test-date]').hasText('a year ago');
    assert.dom('[data-test-invite="nanomsg"] [data-test-accept-button]').exists();
    assert.dom('[data-test-invite="nanomsg"] [data-test-decline-button]').exists();
    assert.dom('[data-test-invite="ember-rs"]').exists();
    assert.dom('[data-test-invite="ember-rs"] [data-test-crate-link]').hasText('ember-rs');
    assert.dom('[data-test-invite="ember-rs"] [data-test-crate-link]').hasAttribute('href', '/crates/ember-rs');
    assert.dom('[data-test-invite="ember-rs"] [data-test-inviter-link]').hasText('wycats');
    assert.dom('[data-test-invite="ember-rs"] [data-test-inviter-link]').hasAttribute('href', '/users/wycats');
    assert.dom('[data-test-invite="ember-rs"] [data-test-date]').hasText('in 3 years');
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
    assert.dom('[data-test-error-message]').containsText('Error in declining invite');
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

    percySnapshot(assert);
  });

  test('error message is shown if accept request fails', async function (assert) {
    prepare(this);

    this.server.put('/api/v1/me/crate_owner_invitations/:crate', () => new Response(500));

    await visit('/me/pending-invites');
    assert.equal(currentURL(), '/me/pending-invites');

    await click('[data-test-invite="nanomsg"] [data-test-accept-button]');
    assert.dom('[data-test-error-message]').containsText('Error in accepting invite');
    assert.dom('[data-test-accepted-message]').doesNotExist();
    assert.dom('[data-test-declined-message]').doesNotExist();
  });
});
