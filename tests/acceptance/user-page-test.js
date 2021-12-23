import { visit } from '@ember/test-helpers';
import { module, test } from 'qunit';

import percySnapshot from '@percy/ember';
import a11yAudit from 'ember-a11y-testing/test-support/audit';

import { setupApplicationTest } from 'cargo/tests/helpers';

import axeConfig from '../axe-config';

module('Acceptance | user page', function (hooks) {
  setupApplicationTest(hooks);

  test('has user display', async function (assert) {
    this.server.loadFixtures();

    await visit('/users/thehydroimpulse');

    assert.dom('[data-test-heading] [data-test-username]').hasText('thehydroimpulse');

    await percySnapshot(assert);
    await a11yAudit(axeConfig);
  });

  test('has link to github in user header', async function (assert) {
    this.server.loadFixtures();

    await visit('/users/thehydroimpulse');

    assert.dom('[data-test-heading] [data-test-user-link]').hasAttribute('href', 'https://github.com/thehydroimpulse');
  });

  test('user details has github profile icon', async function (assert) {
    this.server.loadFixtures();

    await visit('/users/thehydroimpulse');

    assert
      .dom('[data-test-heading] [data-test-avatar]')
      .hasAttribute('src', 'https://avatars.githubusercontent.com/u/565790?v=3&s=170');
  });
});
