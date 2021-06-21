import { click, fillIn, visit } from '@ember/test-helpers';
import { module, test } from 'qunit';

import { setupApplicationTest } from 'cargo/tests/helpers';

module('Acceptance | Settings | Add Owner', function (hooks) {
  setupApplicationTest(hooks);

  function prepare(context) {
    let { server } = context;

    let user1 = server.create('user', { name: 'blabaere' });
    let user2 = server.create('user', { name: 'thehydroimpulse' });
    let team1 = server.create('team', { org: 'org', name: 'blabaere' });
    let team2 = server.create('team', { org: 'org', name: 'thehydroimpulse' });

    let crate = server.create('crate', { name: 'nanomsg' });
    server.create('version', { crate, num: '1.0.0' });
    server.create('crate-ownership', { crate, user: user1 });
    server.create('crate-ownership', { crate, user: user2 });
    server.create('crate-ownership', { crate, team: team1 });
    server.create('crate-ownership', { crate, team: team2 });

    context.authenticateAs(user1);

    return { crate, team1, team2, user1, user2 };
  }

  test('attempting to add owner without username', async function (assert) {
    prepare(this);

    await visit('/crates/nanomsg/settings');
    await fillIn('input[name="username"]', '');
    assert.dom('[data-test-save-button]').isDisabled();
  });

  test('attempting to add non-existent owner', async function (assert) {
    prepare(this);

    await visit('/crates/nanomsg/settings');
    await fillIn('input[name="username"]', 'spookyghostboo');
    await click('[data-test-save-button]');

    assert
      .dom('[data-test-notification-message="error"]')
      .hasText('Error sending invite: could not find user with login `spookyghostboo`');
    assert.dom('[data-test-owners] [data-test-owner-team]').exists({ count: 2 });
    assert.dom('[data-test-owners] [data-test-owner-user]').exists({ count: 2 });
  });

  test('add a new owner', async function (assert) {
    prepare(this);

    this.server.create('user', { name: 'iain8' });

    await visit('/crates/nanomsg/settings');
    await fillIn('input[name="username"]', 'iain8');
    await click('[data-test-save-button]');

    assert.dom('[data-test-notification-message="success"]').hasText('An invite has been sent to iain8');
    assert.dom('[data-test-owners] [data-test-owner-team]').exists({ count: 2 });
    assert.dom('[data-test-owners] [data-test-owner-user]').exists({ count: 2 });
  });
});
