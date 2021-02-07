import { visit } from '@ember/test-helpers';
import { module, test } from 'qunit';

import percySnapshot from '@percy/ember';
import a11yAudit from 'ember-a11y-testing/test-support/audit';

import { setupApplicationTest } from 'cargo/tests/helpers';

import axeConfig from '../axe-config';

module('Acceptance | categories', function (hooks) {
  setupApplicationTest(hooks);

  test('listing categories', async function (assert) {
    this.owner.lookup('service:intl').locale = 'en';

    this.server.create('category', { category: 'API bindings' });
    this.server.create('category', { category: 'Algorithms' });
    this.server.createList('crate', 1, { categoryIds: ['algorithms'] });
    this.server.create('category', { category: 'Asynchronous' });
    this.server.createList('crate', 15, { categoryIds: ['asynchronous'] });
    this.server.create('category', { category: 'Everything', crates_cnt: 1234 });

    await visit('/categories');

    assert.dom('[data-test-category="api-bindings"] [data-test-crate-count]').hasText('0 crates');
    assert.dom('[data-test-category="algorithms"] [data-test-crate-count]').hasText('1 crate');
    assert.dom('[data-test-category="asynchronous"] [data-test-crate-count]').hasText('15 crates');
    assert.dom('[data-test-category="everything"] [data-test-crate-count]').hasText('1,234 crates');

    await percySnapshot(assert);
    await a11yAudit(axeConfig);
  });

  test('listing categories (locale: de)', async function (assert) {
    this.owner.lookup('service:intl').locale = 'de';

    this.server.create('category', { category: 'Everything', crates_cnt: 1234 });

    await visit('/categories');
    assert.dom('[data-test-category="everything"] [data-test-crate-count]').hasText('1.234 crates');
  });

  test('category/:category_id index default sort is recent-downloads', async function (assert) {
    this.server.create('category', { category: 'Algorithms' });

    await visit('/categories/algorithms');

    assert.dom('[data-test-category-sort] [data-test-current-order]').hasText('Recent Downloads');

    await percySnapshot(assert);
    await a11yAudit(axeConfig);
  });

  test('listing category slugs', async function (assert) {
    this.server.create('category', { category: 'Algorithms', description: 'Crates for algorithms' });
    this.server.create('category', { category: 'Asynchronous', description: 'Async crates' });

    await visit('/category_slugs');

    assert.dom('[data-test-category-slug="algorithms"]').hasText('algorithms');
    assert.dom('[data-test-category-description="algorithms"]').hasText('Crates for algorithms');
    assert.dom('[data-test-category-slug="asynchronous"]').hasText('asynchronous');
    assert.dom('[data-test-category-description="asynchronous"]').hasText('Async crates');
  });
});
