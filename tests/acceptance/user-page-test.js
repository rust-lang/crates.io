import { module, test } from 'qunit';
import { setupApplicationTest } from 'ember-qunit';
import { visit } from '@ember/test-helpers';
import a11yAudit from 'ember-a11y-testing/test-support/audit';
import axeConfig from '../axe-config';
import setupMirage from '../helpers/setup-mirage';
import { percySnapshot } from 'ember-percy';

module('Acceptance | user page', function(hooks) {
  setupApplicationTest(hooks);
  setupMirage(hooks);

  test('is accessible', async function(assert) {
    assert.expect(0);

    this.server.loadFixtures();

    await visit('/users/thehydroimpulse');
    percySnapshot(assert);

    await a11yAudit(axeConfig);
  });

  test('has user display', async function(assert) {
    this.server.loadFixtures();

    await visit('/users/thehydroimpulse');

    assert.dom('[data-test-heading] [data-test-username]').hasText('thehydroimpulse');
  });

  test('has link to github in user header', async function(assert) {
    this.server.loadFixtures();

    await visit('/users/thehydroimpulse');

    assert.dom('[data-test-heading] [data-test-user-link]').hasAttribute('href', 'https://github.com/thehydroimpulse');
  });

  test('github link has image in user header', async function(assert) {
    this.server.loadFixtures();

    await visit('/users/thehydroimpulse');

    assert.dom('[data-test-heading] [data-test-user-link] img').hasAttribute('src', '/assets/GitHub-Mark.svg');
  });

  test('user details has github profile icon', async function(assert) {
    this.server.loadFixtures();

    await visit('/users/thehydroimpulse');

    assert
      .dom('[data-test-heading] [data-test-avatar]')
      .hasAttribute('src', 'https://avatars.githubusercontent.com/u/565790?v=3&s=170');
  });
});
