import { visit } from '@ember/test-helpers';
import { module, test } from 'qunit';

import percySnapshot from '@percy/ember';
import a11yAudit from 'ember-a11y-testing/test-support/audit';

import { setupApplicationTest } from 'cargo/tests/helpers';

import axeConfig from '../axe-config';

module('Acceptance | team page', function (hooks) {
  setupApplicationTest(hooks);

  test('has team organization display', async function (assert) {
    this.server.loadFixtures();

    await visit('/teams/github:org:thehydroimpulse');

    assert.dom('[data-test-heading] [data-test-org-name]').hasText('org');
    assert.dom('[data-test-heading] [data-test-team-name]').hasText('thehydroimpulseteam');

    await percySnapshot(assert);
    await a11yAudit(axeConfig);
  });

  test('has link to github in team header', async function (assert) {
    this.server.loadFixtures();

    await visit('/teams/github:org:thehydroimpulse');

    assert.dom('[data-test-heading] [data-test-github-link]').hasAttribute('href', 'https://github.com/org_test');
  });

  test('team organization details has github profile icon', async function (assert) {
    this.server.loadFixtures();

    await visit('/teams/github:org:thehydroimpulse');

    assert
      .dom('[data-test-heading] [data-test-avatar]')
      .hasAttribute('src', 'https://avatars.githubusercontent.com/u/565790?v=3&s=170');
  });
});
