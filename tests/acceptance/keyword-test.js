import { visit } from '@ember/test-helpers';
import { setupApplicationTest } from 'ember-qunit';
import { module, test } from 'qunit';

import a11yAudit from 'ember-a11y-testing/test-support/audit';
import { percySnapshot } from 'ember-percy';

import axeConfig from '../axe-config';
import setupMirage from '../helpers/setup-mirage';

module('Acceptance | keywords', function (hooks) {
  setupApplicationTest(hooks);
  setupMirage(hooks);

  test('keyword/:keyword_id is accessible', async function (assert) {
    assert.expect(0);

    this.server.create('keyword', { keyword: 'network' });

    await visit('keywords/network');
    percySnapshot(assert);

    await a11yAudit(axeConfig);
  });

  test('keyword/:keyword_id index default sort is recent-downloads', async function (assert) {
    this.server.create('keyword', { keyword: 'network' });

    await visit('/keywords/network');

    assert.dom('[data-test-keyword-sort] [data-test-current-order]').hasText('Recent Downloads');
  });
});
