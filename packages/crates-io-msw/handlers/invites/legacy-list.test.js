import { assert, test } from 'vitest';

import { db } from '../../index.js';

test('empty case', async function () {
  let user = db.user.create();
  db.mswSession.create({ user });

  let response = await fetch('/api/v1/me/crate_owner_invitations');
  assert.strictEqual(response.status, 200);
  assert.deepEqual(await response.json(), { crate_owner_invitations: [], users: [] });
});

test('returns the list of invitations for the authenticated user', async function () {
  let nanomsg = db.crate.create({ name: 'nanomsg' });
  db.version.create({ crate: nanomsg });

  let ember = db.crate.create({ name: 'ember-rs' });
  db.version.create({ crate: ember });

  let user = db.user.create();
  db.mswSession.create({ user });

  let inviter = db.user.create({ name: 'janed' });
  db.crateOwnerInvitation.create({
    crate: nanomsg,
    createdAt: '2016-12-24T12:34:56Z',
    invitee: user,
    inviter,
  });

  let inviter2 = db.user.create({ name: 'wycats' });
  db.crateOwnerInvitation.create({
    crate: ember,
    createdAt: '2020-12-31T12:34:56Z',
    invitee: user,
    inviter: inviter2,
  });

  let response = await fetch('/api/v1/me/crate_owner_invitations');
  assert.strictEqual(response.status, 200);
  assert.deepEqual(await response.json(), {
    crate_owner_invitations: [
      {
        crate_id: Number(nanomsg.id),
        crate_name: 'nanomsg',
        created_at: '2016-12-24T12:34:56Z',
        expires_at: '2017-01-24T12:34:56Z',
        invitee_id: Number(user.id),
        inviter_id: Number(inviter.id),
      },
      {
        crate_id: Number(ember.id),
        crate_name: 'ember-rs',
        created_at: '2020-12-31T12:34:56Z',
        expires_at: '2017-01-24T12:34:56Z',
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

test('returns an error if unauthenticated', async function () {
  let response = await fetch('/api/v1/me/crate_owner_invitations');
  assert.strictEqual(response.status, 403);
  assert.deepEqual(await response.json(), {
    errors: [{ detail: 'must be logged in to perform that action' }],
  });
});
