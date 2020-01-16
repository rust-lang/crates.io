import { module, test } from 'qunit';
import { setupApplicationTest } from 'ember-qunit';
import { visit } from '@ember/test-helpers';
import a11yAudit from 'ember-a11y-testing/test-support/audit';
import axeConfig from '../axe-config';
import setupMirage from '../helpers/setup-mirage';
import { percySnapshot } from 'ember-percy';

module('Acceptance | categories', function(hooks) {
  setupApplicationTest(hooks);
  setupMirage(hooks);

  test('is accessible', async function(assert) {
    assert.expect(0);

    this.server.create('category', { category: 'API bindings' });
    this.server.create('category', { category: 'Algorithms' });
    this.server.create('category', { category: 'Asynchronous' });

    await visit('/categories');
    percySnapshot(assert);

    await a11yAudit(axeConfig);
  });

  test('category/:category_id is accessible', async function(assert) {
    assert.expect(0);

    this.server.create('category', { category: 'Algorithms' });

    await visit('/categories/algorithms');
    percySnapshot(assert);

    await a11yAudit(axeConfig);
  });

  test('listing categories', async function(assert) {
    this.server.create('category', { category: 'API bindings' });
    this.server.create('category', { category: 'Algorithms' });
    this.server.createList('crate', 1, { categoryIds: ['algorithms'] });
    this.server.create('category', { category: 'Asynchronous' });
    this.server.createList('crate', 15, { categoryIds: ['asynchronous'] });

    await visit('/categories');

    assert.dom('[data-test-category="api-bindings"] [data-test-crate-count]').hasText('0 crates');
    assert.dom('[data-test-category="algorithms"] [data-test-crate-count]').hasText('1 crate');
    assert.dom('[data-test-category="asynchronous"] [data-test-crate-count]').hasText('15 crates');
  });

  test('category/:category_id index default sort is recent-downloads', async function(assert) {
    this.server.create('category', { category: 'Algorithms' });

    await visit('/categories/algorithms');

    assert.dom('[data-test-category-sort] [data-test-current-order]').hasText('Recent Downloads');
  });

  test('listing category slugs', async function(assert) {
    this.server.create('category', { category: 'Algorithms', description: 'Crates for algorithms' });
    this.server.create('category', { category: 'Asynchronous', description: 'Async crates' });

    await visit('/category_slugs');

    assert.dom('[data-test-category-slug="algorithms"]').hasText('algorithms');
    assert.dom('[data-test-category-description="algorithms"]').hasText('Crates for algorithms');
    assert.dom('[data-test-category-slug="asynchronous"]').hasText('asynchronous');
    assert.dom('[data-test-category-description="asynchronous"]').hasText('Async crates');
  });
});
