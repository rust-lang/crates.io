import { currentURL } from '@ember/test-helpers';
import { module, test } from 'qunit';

import percySnapshot from '@percy/ember';
import Response from 'ember-cli-mirage/response';

import { setupApplicationTest } from 'cargo/tests/helpers';

import setupMirage from '../helpers/setup-mirage';
import { visit } from '../helpers/visit-ignoring-abort';

module('Acceptance | /accept-invite/:token', function (hooks) {
  setupApplicationTest(hooks);
  setupMirage(hooks);

  test('visiting to /accept-invite shows 404 page', async function (assert) {
    await visit('/accept-invite');
    assert.equal(currentURL(), '/accept-invite');
    assert.dom('main').containsText("Oops, that route doesn't exist!");
  });

  test('visiting to /accept-invite/ shows 404 page', async function (assert) {
    await visit('/accept-invite/');
    assert.equal(currentURL(), '/accept-invite/');
    assert.dom('main').containsText("Oops, that route doesn't exist!");
  });

  test('shows error for unknown token', async function (assert) {
    assert.expect(3);

    this.server.put('/api/v1/me/crate_owner_invitations/accept/:token', (schema, request) => {
      assert.deepEqual(request.params, { token: 'unknown' });
      return new Response(404);
    });

    await visit('/accept-invite/unknown');
    assert.equal(currentURL(), '/accept-invite/unknown');
    assert.dom('[data-test-error-message]').hasText('You may want to visit crates.io/me/pending-invites to try again.');
  });

  test('shows success for known token', async function (assert) {
    assert.expect(3);

    this.server.put('/api/v1/me/crate_owner_invitations/accept/:token', (schema, request) => {
      assert.deepEqual(request.params, { token: 'ember-rs' });
      return { crate_owner_invitation: { crate_id: 42, accepted: true } };
    });

    await visit('/accept-invite/ember-rs');
    assert.equal(currentURL(), '/accept-invite/ember-rs');
    assert
      .dom('[data-test-success-message]')
      .hasText(
        'Visit your dashboard to view all of your crates, or account settings to manage email notification preferences for all of your crates.',
      );

    await percySnapshot(assert);
  });
});
