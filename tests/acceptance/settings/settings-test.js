import { currentURL, visit } from '@ember/test-helpers';
import { module, test } from 'qunit';

import percySnapshot from '@percy/ember';
import a11yAudit from 'ember-a11y-testing/test-support/audit';

import { setupApplicationTest } from 'cargo/tests/helpers';

import axeConfig from '../../axe-config';

module('Acceptance | Settings', function (hooks) {
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

  test('listing crate owners', async function (assert) {
    prepare(this);

    await visit('/crates/nanomsg/settings');
    assert.equal(currentURL(), '/crates/nanomsg/settings');

    assert.dom('[data-test-owners] [data-test-owner-team]').exists({ count: 2 });
    assert.dom('[data-test-owners] [data-test-owner-user]').exists({ count: 2 });
    assert.dom('a[href="/teams/github:org:thehydroimpulse"]').exists();
    assert.dom('a[href="/teams/github:org:blabaere"]').exists();
    assert.dom('a[href="/users/thehydroimpulse"]').exists();
    assert.dom('a[href="/users/blabaere"]').exists();

    await percySnapshot(assert);
    await a11yAudit(axeConfig);
  });

  test('/crates/:name/owners redirects to /crates/:name/settings', async function (assert) {
    prepare(this);

    await visit('/crates/nanomsg/owners');
    assert.equal(currentURL(), '/crates/nanomsg/settings');
  });
});
