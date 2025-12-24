import { currentURL, visit } from '@ember/test-helpers';
import { module, test } from 'qunit';

import percySnapshot from '@percy/ember';
import a11yAudit from 'ember-a11y-testing/test-support/audit';

import { setupApplicationTest } from 'crates-io/tests/helpers';

import axeConfig from '../../axe-config';

module('Acceptance | Settings', function (hooks) {
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

  test('listing crate owners', async function (assert) {
    await prepare(this);

    await visit('/crates/nanomsg/settings');
    assert.strictEqual(currentURL(), '/crates/nanomsg/settings');

    assert.dom('[data-test-owners] [data-test-owner-team]').exists({ count: 2 });
    assert.dom('[data-test-owners] [data-test-owner-user]').exists({ count: 2 });
    assert.dom('[data-test-owners] a[href="/teams/github:org:thehydroimpulse"]').exists();
    assert.dom('[data-test-owners] a[href="/teams/github:org:blabaere"]').exists();
    assert.dom('[data-test-owners] a[href="/users/thehydroimpulse"]').exists();
    assert.dom('[data-test-owners] a[href="/users/blabaere"]').exists();

    await percySnapshot(assert);
    await a11yAudit(axeConfig);
  });

  test('/crates/:name/owners redirects to /crates/:name/settings', async function (assert) {
    await prepare(this);

    await visit('/crates/nanomsg/owners');
    assert.strictEqual(currentURL(), '/crates/nanomsg/settings');
  });
});
