import { module, test } from 'qunit';

import fetch from 'fetch';

import { setupTest } from '../../helpers';
import setupMirage from '../../helpers/setup-mirage';

const ADD_USER_BODY = JSON.stringify({ owners: ['john-doe'] });

module('Mirage | PUT /api/v1/crates/:name/owners', function (hooks) {
  setupTest(hooks);
  setupMirage(hooks);

  test('returns 403 if unauthenticated', async function (assert) {
    let response = await fetch('/api/v1/crates/foo/owners', { method: 'PUT', body: ADD_USER_BODY });
    assert.strictEqual(response.status, 403);
    assert.deepEqual(await response.json(), {
      errors: [{ detail: 'must be logged in to perform that action' }],
    });
  });

  test('returns 404 for unknown crates', async function (assert) {
    let user = this.server.create('user');
    this.authenticateAs(user);

    let response = await fetch('/api/v1/crates/foo/owners', { method: 'PUT', body: ADD_USER_BODY });
    assert.strictEqual(response.status, 404);
    assert.deepEqual(await response.json(), { errors: [{ detail: 'Not Found' }] });
  });

  test('can add new owner', async function (assert) {
    let user = this.server.create('user');
    this.authenticateAs(user);

    let crate = this.server.create('crate', { name: 'foo' });
    this.server.create('crate-ownership', { crate, user });

    let user2 = this.server.create('user');

    let body = JSON.stringify({ owners: [user2.login] });
    let response = await fetch('/api/v1/crates/foo/owners', { method: 'PUT', body });
    assert.strictEqual(response.status, 200);
    assert.deepEqual(await response.json(), {
      ok: true,
      msg: 'user user-2 has been invited to be an owner of crate foo',
    });

    let owners = this.server.schema.crateOwnerships.where({ crateId: crate.id });
    assert.strictEqual(owners.length, 1);
    assert.strictEqual(owners.models[0].userId, user.id);

    let invites = this.server.schema.crateOwnerInvitations.where({ crateId: crate.id });
    assert.strictEqual(invites.length, 1);
    assert.strictEqual(invites.models[0].inviterId, user.id);
    assert.strictEqual(invites.models[0].inviteeId, user2.id);
  });

  test('can add team owner', async function (assert) {
    let user = this.server.create('user');
    this.authenticateAs(user);

    let crate = this.server.create('crate', { name: 'foo' });
    this.server.create('crate-ownership', { crate, user });

    let team = this.server.create('team');

    let body = JSON.stringify({ owners: [team.login] });
    let response = await fetch('/api/v1/crates/foo/owners', { method: 'PUT', body });
    assert.strictEqual(response.status, 200);
    assert.deepEqual(await response.json(), {
      ok: true,
      msg: 'team github:rust-lang:team-1 has been added as an owner of crate foo',
    });

    let owners = this.server.schema.crateOwnerships.where({ crateId: crate.id });
    assert.strictEqual(owners.length, 2);
    assert.strictEqual(owners.models[0].userId, user.id);
    assert.strictEqual(owners.models[0].teamId, null);
    assert.strictEqual(owners.models[1].userId, null);
    assert.strictEqual(owners.models[1].teamId, user.id);

    let invites = this.server.schema.crateOwnerInvitations.where({ crateId: crate.id });
    assert.strictEqual(invites.length, 0);
  });

  test('can add multiple owners', async function (assert) {
    let user = this.server.create('user');
    this.authenticateAs(user);

    let crate = this.server.create('crate', { name: 'foo' });
    this.server.create('crate-ownership', { crate, user });

    let team = this.server.create('team');
    let user2 = this.server.create('user');
    let user3 = this.server.create('user');

    let body = JSON.stringify({ owners: [user2.login, team.login, user3.login] });
    let response = await fetch('/api/v1/crates/foo/owners', { method: 'PUT', body });
    assert.strictEqual(response.status, 200);
    assert.deepEqual(await response.json(), {
      ok: true,
      msg: 'user user-2 has been invited to be an owner of crate foo,team github:rust-lang:team-1 has been added as an owner of crate foo,user user-3 has been invited to be an owner of crate foo',
    });

    let owners = this.server.schema.crateOwnerships.where({ crateId: crate.id });
    assert.strictEqual(owners.length, 2);
    assert.strictEqual(owners.models[0].userId, user.id);
    assert.strictEqual(owners.models[0].teamId, null);
    assert.strictEqual(owners.models[1].userId, null);
    assert.strictEqual(owners.models[1].teamId, user.id);

    let invites = this.server.schema.crateOwnerInvitations.where({ crateId: crate.id });
    assert.strictEqual(invites.length, 2);
    assert.strictEqual(invites.models[0].inviterId, user.id);
    assert.strictEqual(invites.models[0].inviteeId, user2.id);
    assert.strictEqual(invites.models[1].inviterId, user.id);
    assert.strictEqual(invites.models[1].inviteeId, user3.id);
  });
});
