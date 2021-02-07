import { visit } from '@ember/test-helpers';
import { module, test } from 'qunit';

import percySnapshot from '@percy/ember';
import a11yAudit from 'ember-a11y-testing/test-support/audit';

import { setupApplicationTest } from 'cargo/tests/helpers';

import axeConfig from '../axe-config';

module('Acceptance | keywords', function (hooks) {
  setupApplicationTest(hooks);

  test('keyword/:keyword_id index default sort is recent-downloads', async function (assert) {
    this.server.create('keyword', { keyword: 'network' });

    await visit('/keywords/network');

    assert.dom('[data-test-keyword-sort] [data-test-current-order]').hasText('Recent Downloads');

    await percySnapshot(assert);
    await a11yAudit(axeConfig);
  });
});
