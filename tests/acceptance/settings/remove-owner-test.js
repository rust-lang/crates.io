import { click, visit } from '@ember/test-helpers';
import { module, test } from 'qunit';

import { setupApplicationTest } from 'cargo/tests/helpers';

module('Acceptance | Settings | Remove Owner', function (hooks) {
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

  test('remove a crate owner when owner is a user', async function (assert) {
    prepare(this);

    await visit('/crates/nanomsg/settings');
    await click('[data-test-owner-user="thehydroimpulse"] [data-test-remove-owner-button]');

    assert.dom('[data-test-notification-message="success"]').hasText('User thehydroimpulse removed as crate owner');
    assert.dom('[data-test-owner-user]').exists({ count: 1 });
  });

  test('remove a user crate owner (error behavior)', async function (assert) {
    let { crate, user2 } = prepare(this);

    // we are intentionally returning a 200 response here, because is what
    // the real backend also returns due to legacy reasons
    this.server.delete('/api/v1/crates/nanomsg/owners', { errors: [{ detail: 'nope' }] });

    await visit(`/crates/${crate.name}/settings`);
    await click(`[data-test-owner-user="${user2.login}"] [data-test-remove-owner-button]`);

    assert
      .dom('[data-test-notification-message="error"]')
      .hasText(`Failed to remove the user ${user2.login} as crate owner: nope`);
    assert.dom('[data-test-owner-user]').exists({ count: 2 });
  });

  test('remove a crate owner when owner is a team', async function (assert) {
    prepare(this);

    await visit('/crates/nanomsg/settings');
    await click('[data-test-owner-team="github:org:thehydroimpulse"] [data-test-remove-owner-button]');

    assert.dom('[data-test-notification-message="success"]').hasText('Team org/thehydroimpulse removed as crate owner');
    assert.dom('[data-test-owner-team]').exists({ count: 1 });
  });

  test('remove a team crate owner (error behavior)', async function (assert) {
    let { crate, team1 } = prepare(this);

    // we are intentionally returning a 200 response here, because is what
    // the real backend also returns due to legacy reasons
    this.server.delete('/api/v1/crates/nanomsg/owners', { errors: [{ detail: 'nope' }] });

    await visit(`/crates/${crate.name}/settings`);
    await click(`[data-test-owner-team="${team1.login}"] [data-test-remove-owner-button]`);

    assert
      .dom('[data-test-notification-message="error"]')
      .hasText(`Failed to remove the team ${team1.org}/${team1.name} as crate owner: nope`);
    assert.dom('[data-test-owner-team]').exists({ count: 2 });
    assert.dom('[data-test-owner-user]').exists({ count: 2 });
  });
});
