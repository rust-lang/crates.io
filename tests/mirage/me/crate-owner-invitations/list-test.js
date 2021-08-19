import { module, test } from 'qunit';

import fetch from 'fetch';

import { setupTest } from '../../../helpers';
import setupMirage from '../../../helpers/setup-mirage';

module('Mirage | GET /api/v1/me/crate_owner_invitations', function (hooks) {
  setupTest(hooks);
  setupMirage(hooks);

  test('empty case', async function (assert) {
    let user = this.server.create('user');
    this.server.create('mirage-session', { user });

    let response = await fetch('/api/v1/me/crate_owner_invitations');
    assert.equal(response.status, 200);
    assert.deepEqual(await response.json(), { crate_owner_invitations: [] });
  });

  test('returns the list of invitations for the authenticated user', async function (assert) {
    let nanomsg = this.server.create('crate', { name: 'nanomsg' });
    this.server.create('version', { crate: nanomsg });

    let ember = this.server.create('crate', { name: 'ember-rs' });
    this.server.create('version', { crate: ember });

    let user = this.server.create('user');
    this.server.create('mirage-session', { user });

    let inviter = this.server.create('user', { name: 'janed' });
    this.server.create('crate-owner-invitation', {
      crate: nanomsg,
      createdAt: '2016-12-24T12:34:56Z',
      invitee: user,
      inviter,
    });

    let inviter2 = this.server.create('user', { name: 'wycats' });
    this.server.create('crate-owner-invitation', {
      crate: ember,
      createdAt: '2020-12-31T12:34:56Z',
      invitee: user,
      inviter: inviter2,
    });

    let response = await fetch('/api/v1/me/crate_owner_invitations');
    assert.equal(response.status, 200);
    assert.deepEqual(await response.json(), {
      crate_owner_invitations: [
        {
          crate_id: Number(nanomsg.id),
          crate_name: 'nanomsg',
          created_at: '2016-12-24T12:34:56Z',
          invited_by_username: 'janed',
          invitee_id: Number(user.id),
          inviter_id: Number(inviter.id),
        },
        {
          crate_id: Number(ember.id),
          crate_name: 'ember-rs',
          created_at: '2020-12-31T12:34:56Z',
          invited_by_username: 'wycats',
          invitee_id: Number(user.id),
          inviter_id: Number(inviter2.id),
        },
      ],
      users: [
        {
          avatar: user.avatar,
          id: Number(user.id),
          login: user.login,
          name: user.name,
          url: user.url,
        },
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
    assert.deepEqual(await response.json(), {
      errors: [{ detail: 'must be logged in to perform that action' }],
    });
  });
});
