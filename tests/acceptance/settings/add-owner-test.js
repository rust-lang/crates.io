import { click, fillIn, visit } from '@ember/test-helpers';
import { module, test } from 'qunit';

import { setupApplicationTest } from 'crates-io/tests/helpers';

module('Acceptance | Settings | Add Owner', function (hooks) {
  setupApplicationTest(hooks);

  async function prepare(context) {
    let { db } = context;

    let user1 = await db.user.create({ name: 'blabaere' });
    let user2 = await db.user.create({ name: 'thehydroimpulse' });
    let team1 = await db.team.create({ org: 'org', name: 'blabaere' });
    let team2 = await db.team.create({ org: 'org', name: 'thehydroimpulse' });

    let crate = await db.crate.create({ name: 'nanomsg' });
    await db.version.create({ crate, num: '1.0.0' });
    await db.crateOwnership.create({ crate, user: user1 });
    await db.crateOwnership.create({ crate, user: user2 });
    await db.crateOwnership.create({ crate, team: team1 });
    await db.crateOwnership.create({ crate, team: team2 });

    await context.authenticateAs(user1);

    return { crate, team1, team2, user1, user2 };
  }

  test('attempting to add owner without username', async function (assert) {
    await prepare(this);

    await visit('/crates/nanomsg/settings');
    await click('[data-test-add-owner-button]');
    await fillIn('input[name="username"]', '');
    assert.dom('[data-test-save-button]').isDisabled();
  });

  test('attempting to add non-existent owner', async function (assert) {
    await prepare(this);

    await visit('/crates/nanomsg/settings');
    await click('[data-test-add-owner-button]');
    await fillIn('input[name="username"]', 'spookyghostboo');
    await click('[data-test-save-button]');

    assert
      .dom('[data-test-notification-message="error"]')
      .hasText('Error sending invite: could not find user with login `spookyghostboo`');
    assert.dom('[data-test-owners] [data-test-owner-team]').exists({ count: 2 });
    assert.dom('[data-test-owners] [data-test-owner-user]').exists({ count: 2 });
  });

  test('add a new owner', async function (assert) {
    await prepare(this);

    await this.db.user.create({ name: 'iain8' });

    await visit('/crates/nanomsg/settings');
    await click('[data-test-add-owner-button]');
    await fillIn('input[name="username"]', 'iain8');
    await click('[data-test-save-button]');

    assert.dom('[data-test-notification-message="success"]').hasText('An invite has been sent to iain8');
    assert.dom('[data-test-owners] [data-test-owner-team]').exists({ count: 2 });
    assert.dom('[data-test-owners] [data-test-owner-user]').exists({ count: 2 });
  });

  test('add a team owner', async function (assert) {
    await prepare(this);

    await this.db.user.create({ name: 'iain8' });
    await this.db.team.create({ org: 'rust-lang', name: 'crates-io' });

    await visit('/crates/nanomsg/settings');
    await click('[data-test-add-owner-button]');
    await fillIn('input[name="username"]', 'github:rust-lang:crates-io');
    await click('[data-test-save-button]');

    assert
      .dom('[data-test-notification-message="success"]')
      .hasText('Team github:rust-lang:crates-io was added as a crate owner');
    assert.dom('[data-test-owners] [data-test-owner-team]').exists({ count: 3 });
    assert.dom('[data-test-owners] [data-test-owner-user]').exists({ count: 2 });
  });
});
