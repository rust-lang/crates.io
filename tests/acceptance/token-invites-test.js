import { currentURL } from '@ember/test-helpers';
import { module, test } from 'qunit';

import percySnapshot from '@percy/ember';
import { http, HttpResponse } from 'msw';

import { setupApplicationTest } from 'crates-io/tests/helpers';

import { visit } from '../helpers/visit-ignoring-abort';

module('Acceptance | /accept-invite/:token', function (hooks) {
  setupApplicationTest(hooks, { msw: true });

  test('visiting to /accept-invite shows 404 page', async function (assert) {
    await visit('/accept-invite');
    assert.strictEqual(currentURL(), '/accept-invite');
    assert.dom('[data-test-404-page]').exists();
    assert.dom('[data-test-title]').hasText('Page not found');
  });

  test('visiting to /accept-invite/ shows 404 page', async function (assert) {
    await visit('/accept-invite/');
    assert.strictEqual(currentURL(), '/accept-invite/');
    assert.dom('[data-test-404-page]').exists();
    assert.dom('[data-test-title]').hasText('Page not found');
  });

  test('shows error for unknown token', async function (assert) {
    let error = HttpResponse.json({}, { status: 404 });
    this.worker.use(http.put('/api/v1/me/crate_owner_invitations/accept/:token', () => error));

    await visit('/accept-invite/unknown');
    assert.strictEqual(currentURL(), '/accept-invite/unknown');
    assert.dom('[data-test-error-message]').hasText('You may want to visit crates.io/me/pending-invites to try again.');
  });

  test('shows error for expired token', async function (assert) {
    let errorMessage =
      'The invitation to become an owner of the demo_crate crate expired. Please reach out to an owner of the crate to request a new invitation.';
    let payload = { errors: [{ detail: errorMessage }] };
    let error = HttpResponse.json(payload, { status: 410 });
    this.worker.use(http.put('/api/v1/me/crate_owner_invitations/accept/:token', () => error));

    await visit('/accept-invite/secret123');
    assert.strictEqual(currentURL(), '/accept-invite/secret123');
    assert.dom('[data-test-error-message]').hasText(errorMessage);
  });

  test('shows success for known token', async function (assert) {
    let inviter = this.db.user.create();
    let invitee = this.db.user.create();

    let crate = this.db.crate.create({ name: 'nanomsg' });
    this.db.version.create({ crate });
    let invite = this.db.crateOwnerInvitation.create({ crate, invitee, inviter });

    await visit(`/accept-invite/${invite.token}`);
    assert.strictEqual(currentURL(), `/accept-invite/${invite.token}`);
    assert
      .dom('[data-test-success-message]')
      .hasText(
        'Visit your dashboard to view all of your crates, or account settings to manage email notification preferences for all of your crates.',
      );

    await percySnapshot(assert);
  });
});
