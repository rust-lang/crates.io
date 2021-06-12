import { module, test } from 'qunit';

import fetch from 'fetch';
import timekeeper from 'timekeeper';

import { setupTest } from 'cargo/tests/helpers';

import setupMirage from '../helpers/setup-mirage';

module('Mirage | Crate Owner Invitations', function (hooks) {
  setupTest(hooks);
  setupMirage(hooks);

  module('GET /api/v1/me/crate_owner_invitations', function () {
    test('empty case', async function (assert) {
      let user = this.server.create('user');
      this.server.create('mirage-session', { user });

      let response = await fetch('/api/v1/me/crate_owner_invitations');
      assert.equal(response.status, 200);

      let responsePayload = await response.json();
      assert.deepEqual(responsePayload, { crate_owner_invitations: [] });
    });

    test('returns a paginated crates list', async function (assert) {
      timekeeper.freeze(new Date('2016-12-24T12:34:56Z'));

      let user = this.server.create('user');
      this.server.create('mirage-session', { user });

      let inviter = this.server.create('user', { name: 'janed' });
      let inviter2 = this.server.create('user', { name: 'wycats' });
      this.server.get('/api/v1/me/crate_owner_invitations', function () {
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

      let response = await fetch('/api/v1/me/crate_owner_invitations');
      assert.equal(response.status, 200);

      let responsePayload = await response.json();
      assert.deepEqual(responsePayload, {
        crate_owner_invitations: [
          {
            crate_id: 42,
            crate_name: 'nanomsg',
            created_at: '2016-12-24T12:34:56Z',
            invited_by_username: 'janed',
            invitee_id: Number(user.id),
            inviter_id: Number(inviter.id),
          },
          {
            crate_id: 1,
            crate_name: 'ember-rs',
            created_at: '2020-12-31T12:34:56Z',
            invited_by_username: 'wycats',
            invitee_id: Number(user.id),
            inviter_id: Number(inviter2.id),
          },
        ],
        users: [
          {
            avatar: 'https://avatars1.githubusercontent.com/u/14631425?v=4',
            id: Number(inviter.id),
            login: 'janed',
            name: 'janed',
            url: 'https://github.com/janed',
          },
          {
            avatar: 'https://avatars1.githubusercontent.com/u/14631425?v=4',
            id: Number(inviter2.id),
            login: 'wycats',
            name: 'wycats',
            url: 'https://github.com/wycats',
          },
        ],
      });
    });

    test('returns an error if unauthenticated', async function (assert) {
      let response = await fetch('/api/v1/me/crate_owner_invitations');
      assert.equal(response.status, 403);

      let responsePayload = await response.json();
      assert.deepEqual(responsePayload, {
        errors: [{ detail: 'must be logged in to perform that action' }],
      });
    });
  });
});
