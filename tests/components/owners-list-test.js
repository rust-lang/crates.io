import { render } from '@ember/test-helpers';
import { module, test } from 'qunit';

import { hbs } from 'ember-cli-htmlbars';

import { setupRenderingTest } from 'cargo/tests/helpers';

import setupMirage from '../helpers/setup-mirage';

module('Component | OwnersList', function (hooks) {
  setupRenderingTest(hooks);
  setupMirage(hooks);

  test('single user', async function (assert) {
    let crate = this.server.create('crate');
    this.server.create('version', { crate });

    let user = this.server.create('user');
    this.server.create('crate-ownership', { crate, user });

    let store = this.owner.lookup('service:store');
    this.crate = await store.findRecord('crate', crate.name);
    await this.crate.hasMany('owner_team').load();
    await this.crate.hasMany('owner_user').load();

    await render(hbs`<OwnersList @owners={{this.crate.owners}} />`);
    assert.dom('[data-test-owners="detailed"]').exists();
    assert.dom('ul > li').exists({ count: 1 });
    assert.dom('[data-test-owner-link]').exists({ count: 1 });

    let logins = [...this.element.querySelectorAll('[data-test-owner-link]')].map(it => it.dataset.testOwnerLink);
    assert.deepEqual(logins, ['user-1']);

    assert.dom('[data-test-owner-link="user-1"]').hasText('User 1');
    assert.dom('[data-test-owner-link="user-1"]').hasAttribute('href', '/users/user-1');
  });

  test('user without `name`', async function (assert) {
    let crate = this.server.create('crate');
    this.server.create('version', { crate });

    let user = this.server.create('user', { name: null, login: 'anonymous' });
    this.server.create('crate-ownership', { crate, user });

    let store = this.owner.lookup('service:store');
    this.crate = await store.findRecord('crate', crate.name);
    await this.crate.hasMany('owner_team').load();
    await this.crate.hasMany('owner_user').load();

    await render(hbs`<OwnersList @owners={{this.crate.owners}} />`);
    assert.dom('[data-test-owners="detailed"]').exists();
    assert.dom('ul > li').exists({ count: 1 });
    assert.dom('[data-test-owner-link]').exists({ count: 1 });

    let logins = [...this.element.querySelectorAll('[data-test-owner-link]')].map(it => it.dataset.testOwnerLink);
    assert.deepEqual(logins, ['anonymous']);

    assert.dom('[data-test-owner-link="anonymous"]').hasText('anonymous');
    assert.dom('[data-test-owner-link="anonymous"]').hasAttribute('href', '/users/anonymous');
  });

  test('five users', async function (assert) {
    let crate = this.server.create('crate');
    this.server.create('version', { crate });

    for (let i = 0; i < 5; i++) {
      let user = this.server.create('user');
      this.server.create('crate-ownership', { crate, user });
    }

    let store = this.owner.lookup('service:store');
    this.crate = await store.findRecord('crate', crate.name);
    await this.crate.hasMany('owner_team').load();
    await this.crate.hasMany('owner_user').load();

    await render(hbs`<OwnersList @owners={{this.crate.owners}} />`);
    assert.dom('[data-test-owners="detailed"]').exists();
    assert.dom('ul > li').exists({ count: 5 });
    assert.dom('[data-test-owner-link]').exists({ count: 5 });

    let logins = [...this.element.querySelectorAll('[data-test-owner-link]')].map(it => it.dataset.testOwnerLink);
    assert.deepEqual(logins, ['user-1', 'user-2', 'user-3', 'user-4', 'user-5']);
  });

  test('six users', async function (assert) {
    let crate = this.server.create('crate');
    this.server.create('version', { crate });

    for (let i = 0; i < 6; i++) {
      let user = this.server.create('user');
      this.server.create('crate-ownership', { crate, user });
    }

    let store = this.owner.lookup('service:store');
    this.crate = await store.findRecord('crate', crate.name);
    await this.crate.hasMany('owner_team').load();
    await this.crate.hasMany('owner_user').load();

    await render(hbs`<OwnersList @owners={{this.crate.owners}} />`);
    assert.dom('[data-test-owners="basic"]').exists();
    assert.dom('ul > li').exists({ count: 6 });
    assert.dom('[data-test-owner-link]').exists({ count: 6 });

    let logins = [...this.element.querySelectorAll('[data-test-owner-link]')].map(it => it.dataset.testOwnerLink);
    assert.deepEqual(logins, ['user-1', 'user-2', 'user-3', 'user-4', 'user-5', 'user-6']);
  });

  test('teams mixed with users', async function (assert) {
    let crate = this.server.create('crate');
    this.server.create('version', { crate });

    for (let i = 0; i < 3; i++) {
      let user = this.server.create('user');
      this.server.create('crate-ownership', { crate, user });
    }
    for (let i = 0; i < 2; i++) {
      let team = this.server.create('team', { org: 'crates-io' });
      this.server.create('crate-ownership', { crate, team });
    }

    let store = this.owner.lookup('service:store');
    this.crate = await store.findRecord('crate', crate.name);
    await this.crate.hasMany('owner_team').load();
    await this.crate.hasMany('owner_user').load();

    await render(hbs`<OwnersList @owners={{this.crate.owners}} />`);
    assert.dom('[data-test-owners="detailed"]').exists();
    assert.dom('ul > li').exists({ count: 5 });
    assert.dom('[data-test-owner-link]').exists({ count: 5 });

    let logins = [...this.element.querySelectorAll('[data-test-owner-link]')].map(it => it.dataset.testOwnerLink);
    assert.deepEqual(logins, ['github:crates-io:team-1', 'github:crates-io:team-2', 'user-1', 'user-2', 'user-3']);

    assert.dom('[data-test-owner-link="github:crates-io:team-1"]').hasText('crates-io/team-1');
    assert
      .dom('[data-test-owner-link="github:crates-io:team-1"]')
      .hasAttribute('href', '/teams/github:crates-io:team-1');
  });
});
