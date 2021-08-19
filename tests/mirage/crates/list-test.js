import { module, test } from 'qunit';

import fetch from 'fetch';

import { setupTest } from '../../helpers';
import setupMirage from '../../helpers/setup-mirage';

module('Mirage | GET /api/v1/crates', function (hooks) {
  setupTest(hooks);
  setupMirage(hooks);

  test('empty case', async function (assert) {
    let response = await fetch('/api/v1/crates');
    assert.equal(response.status, 200);
    assert.deepEqual(await response.json(), {
      crates: [],
      meta: {
        total: 0,
      },
    });
  });

  test('returns a paginated crates list', async function (assert) {
    let crate = this.server.create('crate', { name: 'rand' });
    this.server.create('version', {
      crate,
      created_at: '2020-11-06T12:34:56Z',
      num: '1.0.0',
      updated_at: '2020-11-06T12:34:56Z',
    });
    this.server.create('version', {
      crate,
      created_at: '2020-12-25T12:34:56Z',
      num: '2.0.0-beta.1',
      updated_at: '2020-12-25T12:34:56Z',
    });

    let response = await fetch('/api/v1/crates');
    assert.equal(response.status, 200);
    assert.deepEqual(await response.json(), {
      crates: [
        {
          id: 'rand',
          badges: [],
          categories: [],
          created_at: '2010-06-16T21:30:45Z',
          description: 'This is the description for the crate called "rand"',
          documentation: null,
          downloads: 0,
          homepage: null,
          keywords: [],
          links: {
            owner_team: '/api/v1/crates/rand/owner_team',
            owner_user: '/api/v1/crates/rand/owner_user',
            reverse_dependencies: '/api/v1/crates/rand/reverse_dependencies',
            version_downloads: '/api/v1/crates/rand/downloads',
            versions: '/api/v1/crates/rand/versions',
          },
          max_version: '2.0.0-beta.1',
          max_stable_version: '1.0.0',
          name: 'rand',
          newest_version: '2.0.0-beta.1',
          repository: null,
          updated_at: '2017-02-24T12:34:56Z',
          versions: ['1', '2'],
        },
      ],
      meta: {
        total: 1,
      },
    });
  });

  test('never returns more than 10 results', async function (assert) {
    let crates = this.server.createList('crate', 25);
    this.server.createList('version', crates.length, { crate: i => crates[i] });

    let response = await fetch('/api/v1/crates');
    assert.equal(response.status, 200);

    let responsePayload = await response.json();
    assert.equal(responsePayload.crates.length, 10);
    assert.equal(responsePayload.meta.total, 25);
  });

  test('supports `page` and `per_page` parameters', async function (assert) {
    let crates = this.server.createList('crate', 25, {
      name: i => `crate-${String(i + 1).padStart(2, '0')}`,
    });
    this.server.createList('version', crates.length, { crate: i => crates[i] });

    let response = await fetch('/api/v1/crates?page=2&per_page=5');
    assert.equal(response.status, 200);

    let responsePayload = await response.json();
    assert.equal(responsePayload.crates.length, 5);
    assert.deepEqual(
      responsePayload.crates.map(it => it.id),
      ['crate-06', 'crate-07', 'crate-08', 'crate-09', 'crate-10'],
    );
    assert.equal(responsePayload.meta.total, 25);
  });

  test('supports a `letter` parameter', async function (assert) {
    let foo = this.server.create('crate', { name: 'foo' });
    this.server.create('version', { crate: foo });
    let bar = this.server.create('crate', { name: 'bar' });
    this.server.create('version', { crate: bar });
    let baz = this.server.create('crate', { name: 'BAZ' });
    this.server.create('version', { crate: baz });

    let response = await fetch('/api/v1/crates?letter=b');
    assert.equal(response.status, 200);

    let responsePayload = await response.json();
    assert.equal(responsePayload.crates.length, 2);
    assert.deepEqual(
      responsePayload.crates.map(it => it.id),
      ['bar', 'BAZ'],
    );
    assert.equal(responsePayload.meta.total, 2);
  });

  test('supports a `q` parameter', async function (assert) {
    let crate1 = this.server.create('crate', { name: '123456' });
    this.server.create('version', { crate: crate1 });
    let crate2 = this.server.create('crate', { name: '00123' });
    this.server.create('version', { crate: crate2 });
    let crate3 = this.server.create('crate', { name: '87654' });
    this.server.create('version', { crate: crate3 });

    let response = await fetch('/api/v1/crates?q=123');
    assert.equal(response.status, 200);

    let responsePayload = await response.json();
    assert.equal(responsePayload.crates.length, 2);
    assert.deepEqual(
      responsePayload.crates.map(it => it.id),
      ['123456', '00123'],
    );
    assert.equal(responsePayload.meta.total, 2);
  });

  test('supports a `user_id` parameter', async function (assert) {
    let user1 = this.server.create('user');
    let user2 = this.server.create('user');

    let foo = this.server.create('crate', { name: 'foo' });
    this.server.create('version', { crate: foo });
    let bar = this.server.create('crate', { name: 'bar' });
    this.server.create('crate-ownership', { crate: bar, user: user1 });
    this.server.create('version', { crate: bar });
    let baz = this.server.create('crate', { name: 'baz' });
    this.server.create('crate-ownership', { crate: baz, user: user2 });
    this.server.create('version', { crate: baz });

    let response = await fetch(`/api/v1/crates?user_id=${user1.id}`);
    assert.equal(response.status, 200);

    let responsePayload = await response.json();
    assert.equal(responsePayload.crates.length, 1);
    assert.equal(responsePayload.crates[0].id, 'bar');
    assert.equal(responsePayload.meta.total, 1);
  });

  test('supports a `team_id` parameter', async function (assert) {
    let team1 = this.server.create('team');
    let team2 = this.server.create('team');

    let foo = this.server.create('crate', { name: 'foo' });
    this.server.create('version', { crate: foo });
    let bar = this.server.create('crate', { name: 'bar' });
    this.server.create('crate-ownership', { crate: bar, team: team1 });
    this.server.create('version', { crate: bar });
    let baz = this.server.create('crate', { name: 'baz' });
    this.server.create('crate-ownership', { crate: baz, team: team2 });
    this.server.create('version', { crate: baz });

    let response = await fetch(`/api/v1/crates?team_id=${team1.id}`);
    assert.equal(response.status, 200);

    let responsePayload = await response.json();
    assert.equal(responsePayload.crates.length, 1);
    assert.equal(responsePayload.crates[0].id, 'bar');
    assert.equal(responsePayload.meta.total, 1);
  });

  test('supports a `following` parameter', async function (assert) {
    let foo = this.server.create('crate', { name: 'foo' });
    this.server.create('version', { crate: foo });
    let bar = this.server.create('crate', { name: 'bar' });
    this.server.create('version', { crate: bar });

    let user = this.server.create('user', { followedCrates: [bar] });
    this.authenticateAs(user);

    let response = await fetch(`/api/v1/crates?following=1`);
    assert.equal(response.status, 200);

    let responsePayload = await response.json();
    assert.equal(responsePayload.crates.length, 1);
    assert.equal(responsePayload.crates[0].id, 'bar');
    assert.equal(responsePayload.meta.total, 1);
  });

  test('supports multiple `ids[]` parameters', async function (assert) {
    let foo = this.server.create('crate', { name: 'foo' });
    this.server.create('version', { crate: foo });
    let bar = this.server.create('crate', { name: 'bar' });
    this.server.create('version', { crate: bar });
    let baz = this.server.create('crate', { name: 'baz' });
    this.server.create('version', { crate: baz });
    let other = this.server.create('crate', { name: 'other' });
    this.server.create('version', { crate: other });

    let response = await fetch(`/api/v1/crates?ids[]=foo&ids[]=bar&ids[]=baz&ids[]=baz&ids[]=unknown`);
    assert.equal(response.status, 200);

    let responsePayload = await response.json();
    assert.equal(responsePayload.crates.length, 3);
    assert.equal(responsePayload.crates[0].id, 'foo');
    assert.equal(responsePayload.crates[1].id, 'bar');
    assert.equal(responsePayload.crates[2].id, 'baz');
    assert.equal(responsePayload.meta.total, 3);
  });
});
