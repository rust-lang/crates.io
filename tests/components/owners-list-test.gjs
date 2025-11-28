import { render } from '@ember/test-helpers';
import { module, test } from 'qunit';

import OwnersList from 'crates-io/components/owners-list';
import { setupRenderingTest } from 'crates-io/tests/helpers';
import setupMsw from 'crates-io/tests/helpers/setup-msw';

module('Component | OwnersList', function (hooks) {
  setupRenderingTest(hooks);
  setupMsw(hooks);

  test('single user', async function (assert) {
    let crate = await this.db.crate.create();
    await this.db.version.create({ crate });

    let user = await this.db.user.create();
    await this.db.crateOwnership.create({ crate, user });

    let store = this.owner.lookup('service:store');
    let crateModel = await store.findRecord('crate', crate.name);
    await crateModel.loadOwnersTask.perform();

    await render(<template><OwnersList @owners={{crateModel.owners}} /></template>);
    assert.dom('[data-test-owners="detailed"]').exists();
    assert.dom('ul > li').exists({ count: 1 });
    assert.dom('[data-test-owner-link]').exists({ count: 1 });

    let logins = [...this.element.querySelectorAll('[data-test-owner-link]')].map(it => it.dataset.testOwnerLink);
    assert.deepEqual(logins, ['user-1']);

    assert.dom('[data-test-owner-link="user-1"]').hasText('User 1');
    assert.dom('[data-test-owner-link="user-1"]').hasAttribute('href', '/users/user-1');
  });

  test('user without `name`', async function (assert) {
    let crate = await this.db.crate.create();
    await this.db.version.create({ crate });

    let user = await this.db.user.create({ name: null, login: 'anonymous' });
    await this.db.crateOwnership.create({ crate, user });

    let store = this.owner.lookup('service:store');
    let crateModel = await store.findRecord('crate', crate.name);
    await crateModel.loadOwnersTask.perform();

    await render(<template><OwnersList @owners={{crateModel.owners}} /></template>);
    assert.dom('[data-test-owners="detailed"]').exists();
    assert.dom('ul > li').exists({ count: 1 });
    assert.dom('[data-test-owner-link]').exists({ count: 1 });

    let logins = [...this.element.querySelectorAll('[data-test-owner-link]')].map(it => it.dataset.testOwnerLink);
    assert.deepEqual(logins, ['anonymous']);

    assert.dom('[data-test-owner-link="anonymous"]').hasText('anonymous');
    assert.dom('[data-test-owner-link="anonymous"]').hasAttribute('href', '/users/anonymous');
  });

  test('five users', async function (assert) {
    let crate = await this.db.crate.create();
    await this.db.version.create({ crate });

    for (let i = 0; i < 5; i++) {
      let user = await this.db.user.create();
      await this.db.crateOwnership.create({ crate, user });
    }

    let store = this.owner.lookup('service:store');
    let crateModel = await store.findRecord('crate', crate.name);
    await crateModel.loadOwnersTask.perform();

    await render(<template><OwnersList @owners={{crateModel.owners}} /></template>);
    assert.dom('[data-test-owners="detailed"]').exists();
    assert.dom('ul > li').exists({ count: 5 });
    assert.dom('[data-test-owner-link]').exists({ count: 5 });

    let logins = [...this.element.querySelectorAll('[data-test-owner-link]')].map(it => it.dataset.testOwnerLink);
    assert.deepEqual(logins, ['user-1', 'user-2', 'user-3', 'user-4', 'user-5']);
  });

  test('six users', async function (assert) {
    let crate = await this.db.crate.create();
    await this.db.version.create({ crate });

    for (let i = 0; i < 6; i++) {
      let user = await this.db.user.create();
      await this.db.crateOwnership.create({ crate, user });
    }

    let store = this.owner.lookup('service:store');
    let crateModel = await store.findRecord('crate', crate.name);
    await crateModel.loadOwnersTask.perform();

    await render(<template><OwnersList @owners={{crateModel.owners}} /></template>);
    assert.dom('[data-test-owners="basic"]').exists();
    assert.dom('ul > li').exists({ count: 6 });
    assert.dom('[data-test-owner-link]').exists({ count: 6 });

    let logins = [...this.element.querySelectorAll('[data-test-owner-link]')].map(it => it.dataset.testOwnerLink);
    assert.deepEqual(logins, ['user-1', 'user-2', 'user-3', 'user-4', 'user-5', 'user-6']);
  });

  test('teams mixed with users', async function (assert) {
    let crate = await this.db.crate.create();
    await this.db.version.create({ crate });

    for (let i = 0; i < 3; i++) {
      let user = await this.db.user.create();
      await this.db.crateOwnership.create({ crate, user });
    }
    for (let i = 0; i < 2; i++) {
      let team = await this.db.team.create({ org: 'crates-io' });
      await this.db.crateOwnership.create({ crate, team });
    }

    let store = this.owner.lookup('service:store');
    let crateModel = await store.findRecord('crate', crate.name);
    await crateModel.loadOwnersTask.perform();

    await render(<template><OwnersList @owners={{crateModel.owners}} /></template>);
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
