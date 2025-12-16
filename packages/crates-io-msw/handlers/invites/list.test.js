import { expect, test } from 'vitest';

import { db } from '../../index.js';

test('happy path (invitee_id)', async function () {
  let nanomsg = await db.crate.create({ name: 'nanomsg' });
  await db.version.create({ crate: nanomsg });

  let ember = await db.crate.create({ name: 'ember-rs' });
  await db.version.create({ crate: ember });

  let user = await db.user.create();
  await db.mswSession.create({ user });

  let inviter = await db.user.create({ name: 'janed' });
  await db.crateOwnerInvitation.create({
    crate: nanomsg,
    createdAt: '2016-12-24T12:34:56Z',
    invitee: user,
    inviter,
  });

  let inviter2 = await db.user.create({ name: 'wycats' });
  await db.crateOwnerInvitation.create({
    crate: ember,
    createdAt: '2020-12-31T12:34:56Z',
    invitee: user,
    inviter: inviter2,
  });

  let response = await fetch(`/api/private/crate_owner_invitations?invitee_id=${user.id}`);
  expect(response.status).toBe(200);
  expect(await response.json()).toEqual({
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
  let user = await db.user.create();
  await db.mswSession.create({ user });

  let response = await fetch(`/api/private/crate_owner_invitations?invitee_id=${user.id}`);
  expect(response.status).toBe(200);
  expect(await response.json()).toEqual({
    crate_owner_invitations: [],
    users: [],
    meta: {
      next_page: null,
    },
  });
});

test('happy path with pagination (invitee_id)', async function () {
  let inviter = await db.user.create();

  let user = await db.user.create();
  await db.mswSession.create({ user });

  for (let i = 0; i < 15; i++) {
    let crate = await db.crate.create();
    await db.version.create({ crate });
    await db.crateOwnerInvitation.create({ crate, invitee: user, inviter });
  }

  let response = await fetch(`/api/private/crate_owner_invitations?invitee_id=${user.id}`);
  expect(response.status).toBe(200);
  let responseJSON = await response.json();
  expect(responseJSON['crate_owner_invitations'].length).toBe(10);
  expect(responseJSON.meta['next_page']).toBeTruthy();

  response = await fetch(`/api/private/crate_owner_invitations${responseJSON.meta['next_page']}`);
  expect(response.status).toBe(200);
  responseJSON = await response.json();
  expect(responseJSON['crate_owner_invitations'].length).toBe(5);
  expect(responseJSON.meta['next_page']).toBe(null);
});

test('happy path (crate_name)', async function () {
  let nanomsg = await db.crate.create({ name: 'nanomsg' });
  await db.version.create({ crate: nanomsg });

  let ember = await db.crate.create({ name: 'ember-rs' });
  await db.version.create({ crate: ember });

  let user = await db.user.create();
  await db.mswSession.create({ user });

  let inviter = await db.user.create({ name: 'janed' });
  await db.crateOwnerInvitation.create({
    crate: nanomsg,
    createdAt: '2016-12-24T12:34:56Z',
    invitee: user,
    inviter,
  });

  let inviter2 = await db.user.create({ name: 'wycats' });
  await db.crateOwnerInvitation.create({
    crate: ember,
    createdAt: '2020-12-31T12:34:56Z',
    invitee: user,
    inviter: inviter2,
  });

  let response = await fetch(`/api/private/crate_owner_invitations?crate_name=ember-rs`);
  expect(response.status).toBe(200);
  expect(await response.json()).toEqual({
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
  expect(response.status).toBe(403);
  expect(await response.json()).toEqual({
    errors: [{ detail: 'must be logged in to perform that action' }],
  });
});

test('returns 400 if query params are missing', async function () {
  let user = await db.user.create();
  await db.mswSession.create({ user });

  let response = await fetch(`/api/private/crate_owner_invitations`);
  expect(response.status).toBe(400);
  expect(await response.json()).toEqual({
    errors: [{ detail: 'missing or invalid filter' }],
  });
});

test("returns 404 if crate can't be found", async function () {
  let user = await db.user.create();
  await db.mswSession.create({ user });

  let response = await fetch(`/api/private/crate_owner_invitations?crate_name=foo`);
  expect(response.status).toBe(404);
  expect(await response.json()).toEqual({
    errors: [{ detail: 'Not Found' }],
  });
});

test('returns 403 if requesting for other user', async function () {
  let user = await db.user.create();
  await db.mswSession.create({ user });

  let response = await fetch(`/api/private/crate_owner_invitations?invitee_id=${user.id + 1}`);
  expect(response.status).toBe(403);
  expect(await response.json()).toEqual({
    errors: [{ detail: 'must be logged in to perform that action' }],
  });
});
