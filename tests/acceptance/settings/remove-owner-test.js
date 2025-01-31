import { click, visit } from '@ember/test-helpers';
import { module, test } from 'qunit';

import { http, HttpResponse } from 'msw';

import { setupApplicationTest } from 'crates-io/tests/helpers';

module('Acceptance | Settings | Remove Owner', function (hooks) {
  setupApplicationTest(hooks, { msw: true });

  function prepare(context) {
    let { db } = context;

    let user1 = db.user.create({ name: 'blabaere' });
    let user2 = db.user.create({ name: 'thehydroimpulse' });
    let team1 = db.team.create({ org: 'org', name: 'blabaere' });
    let team2 = db.team.create({ org: 'org', name: 'thehydroimpulse' });

    let crate = db.crate.create({ name: 'nanomsg' });
    db.version.create({ crate, num: '1.0.0' });
    db.crateOwnership.create({ crate, user: user1 });
    db.crateOwnership.create({ crate, user: user2 });
    db.crateOwnership.create({ crate, team: team1 });
    db.crateOwnership.create({ crate, team: team2 });

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
    this.worker.use(
      http.delete('/api/v1/crates/nanomsg/owners', () => HttpResponse.json({ errors: [{ detail: 'nope' }] })),
    );

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
    this.worker.use(
      http.delete('/api/v1/crates/nanomsg/owners', () => HttpResponse.json({ errors: [{ detail: 'nope' }] })),
    );

    await visit(`/crates/${crate.name}/settings`);
    await click(`[data-test-owner-team="${team1.login}"] [data-test-remove-owner-button]`);

    assert
      .dom('[data-test-notification-message="error"]')
      .hasText(`Failed to remove the team ${team1.org}/${team1.name} as crate owner: nope`);
    assert.dom('[data-test-owner-team]').exists({ count: 2 });
    assert.dom('[data-test-owner-user]').exists({ count: 2 });
  });
});
