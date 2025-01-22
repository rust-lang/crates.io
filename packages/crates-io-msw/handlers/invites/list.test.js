import { assert, test } from 'vitest';

import { db } from '../../index.js';

test('happy path (invitee_id)', async function () {
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

  let response = await fetch(`/api/private/crate_owner_invitations?invitee_id=${user.id}`);
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
    meta: {
      next_page: null,
    },
  });
});

test('happy path with empty response (invitee_id)', async function () {
  let user = db.user.create();
  db.mswSession.create({ user });

  let response = await fetch(`/api/private/crate_owner_invitations?invitee_id=${user.id}`);
  assert.strictEqual(response.status, 200);
  assert.deepEqual(await response.json(), {
    crate_owner_invitations: [],
    users: [],
    meta: {
      next_page: null,
    },
  });
});

test('happy path with pagination (invitee_id)', async function () {
  let inviter = db.user.create();

  let user = db.user.create();
  db.mswSession.create({ user });

  for (let i = 0; i < 15; i++) {
    let crate = db.crate.create();
    db.version.create({ crate });
    db.crateOwnerInvitation.create({ crate, invitee: user, inviter });
  }

  let response = await fetch(`/api/private/crate_owner_invitations?invitee_id=${user.id}`);
  assert.strictEqual(response.status, 200);
  let responseJSON = await response.json();
  assert.strictEqual(responseJSON['crate_owner_invitations'].length, 10);
  assert.ok(responseJSON.meta['next_page']);

  response = await fetch(`/api/private/crate_owner_invitations${responseJSON.meta['next_page']}`);
  assert.strictEqual(response.status, 200);
  responseJSON = await response.json();
  assert.strictEqual(responseJSON['crate_owner_invitations'].length, 5);
  assert.strictEqual(responseJSON.meta['next_page'], null);
});

test('happy path (crate_name)', async function () {
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

  let response = await fetch(`/api/private/crate_owner_invitations?crate_name=ember-rs`);
  assert.strictEqual(response.status, 200);
  assert.deepEqual(await response.json(), {
    crate_owner_invitations: [
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
        id: Number(inviter2.id),
        login: 'wycats',
        name: 'wycats',
        url: 'https://github.com/wycats',
      },
    ],
    meta: {
      next_page: null,
    },
  });
});

test('returns 403 if unauthenticated', async function () {
  let response = await fetch(`/api/private/crate_owner_invitations?invitee_id=42`);
  assert.strictEqual(response.status, 403);
  assert.deepEqual(await response.json(), {
    errors: [{ detail: 'must be logged in to perform that action' }],
  });
});

test('returns 400 if query params are missing', async function () {
  let user = db.user.create();
  db.mswSession.create({ user });

  let response = await fetch(`/api/private/crate_owner_invitations`);
  assert.strictEqual(response.status, 400);
  assert.deepEqual(await response.json(), {
    errors: [{ detail: 'missing or invalid filter' }],
  });
});

test("returns 404 if crate can't be found", async function () {
  let user = db.user.create();
  db.mswSession.create({ user });

  let response = await fetch(`/api/private/crate_owner_invitations?crate_name=foo`);
  assert.strictEqual(response.status, 404);
  assert.deepEqual(await response.json(), {
    errors: [{ detail: 'Not Found' }],
  });
});

test('returns 403 if requesting for other user', async function () {
  let user = db.user.create();
  db.mswSession.create({ user });

  let response = await fetch(`/api/private/crate_owner_invitations?invitee_id=${user.id + 1}`);
  assert.strictEqual(response.status, 403);
  assert.deepEqual(await response.json(), {
    errors: [{ detail: 'must be logged in to perform that action' }],
  });
});
