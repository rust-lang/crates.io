import { visit } from '@ember/test-helpers';
import { setupApplicationTest } from 'ember-qunit';
import { module, test } from 'qunit';

import percySnapshot from '@percy/ember';
import a11yAudit from 'ember-a11y-testing/test-support/audit';

import axeConfig from '../axe-config';
import setupMirage from '../helpers/setup-mirage';

module('Acceptance | keywords', function (hooks) {
  setupApplicationTest(hooks);
  setupMirage(hooks);

  test('keyword/:keyword_id index default sort is recent-downloads', async function (assert) {
    this.server.create('keyword', { keyword: 'network' });

    await visit('/keywords/network');

    assert.dom('[data-test-keyword-sort] [data-test-current-order]').hasText('Recent Downloads');

    await percySnapshot(assert);
    await a11yAudit(axeConfig);
  });
});
