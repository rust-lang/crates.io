import { visit } from '@ember/test-helpers';
import { module, test } from 'qunit';

import percySnapshot from '@percy/ember';
import a11yAudit from 'ember-a11y-testing/test-support/audit';

import { setupApplicationTest } from 'crates-io/tests/helpers';

import axeConfig from '../axe-config';

module('Acceptance | categories', function (hooks) {
  setupApplicationTest(hooks);

  test('listing categories', async function (assert) {
    this.owner.lookup('service:intl').locale = 'en';

    this.db.category.create({ category: 'API bindings' });
    let algos = this.db.category.create({ category: 'Algorithms' });
    this.db.crate.create({ categories: [algos] });
    let async = this.db.category.create({ category: 'Asynchronous' });
    Array.from({ length: 15 }, () => this.db.crate.create({ categories: [async] }));
    this.db.category.create({ category: 'Everything', crates_cnt: 1234 });

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

    this.db.category.create({ category: 'Everything', crates_cnt: 1234 });

    await visit('/categories');
    assert.dom('[data-test-category="everything"] [data-test-crate-count]').hasText('1.234 crates');
  });

  test('category/:category_id index default sort is recent-downloads', async function (assert) {
    this.db.category.create({ category: 'Algorithms' });

    await visit('/categories/algorithms');

    assert.dom('[data-test-category-sort] [data-test-current-order]').hasText('Recent Downloads');

    await percySnapshot(assert);
    await a11yAudit(axeConfig);
  });

  test('listing category slugs', async function (assert) {
    this.db.category.create({ category: 'Algorithms', description: 'Crates for algorithms' });
    this.db.category.create({ category: 'Asynchronous', description: 'Async crates' });

    await visit('/category_slugs');

    assert.dom('[data-test-category-slug="algorithms"]').hasText('algorithms');
    assert.dom('[data-test-category-description="algorithms"]').hasText('Crates for algorithms');
    assert.dom('[data-test-category-slug="asynchronous"]').hasText('asynchronous');
    assert.dom('[data-test-category-description="asynchronous"]').hasText('Async crates');
  });
});
