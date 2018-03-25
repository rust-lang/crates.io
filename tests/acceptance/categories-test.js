import { module, test } from 'qunit';
import { setupApplicationTest } from 'ember-qunit';
import { visit } from 'ember-native-dom-helpers';
import a11yAudit from 'ember-a11y-testing/test-support/audit';
import axeConfig from '../axe-config';

module('Acceptance | categories', function(hooks) {
    setupApplicationTest(hooks);

    test('is accessible', async function(assert) {
        assert.expect(0);

        server.create('category', { category: 'API bindings', crates_cnt: 0 });
        server.create('category', { category: 'Algorithms', crates_cnt: 1 });
        server.create('category', { category: 'Asynchronous', crates_cnt: 3910 });

        await visit('/categories');
        await a11yAudit(axeConfig);
    });

    test('category/:category_id is accessible', async function(assert) {
        assert.expect(0);

        server.create('category', { category: 'Algorithms', crates_cnt: 1 });

        await visit('/categories/algorithms');
        await a11yAudit(axeConfig);
    });

    test('listing categories', async function(assert) {
        server.create('category', { category: 'API bindings', crates_cnt: 0 });
        server.create('category', { category: 'Algorithms', crates_cnt: 1 });
        server.create('category', { category: 'Asynchronous', crates_cnt: 3910 });

        await visit('/categories');

        assert.dom('[data-test-category="api-bindings"] [data-test-crate-count]').hasText('0 crates');
        assert.dom('[data-test-category="algorithms"] [data-test-crate-count]').hasText('1 crate');
        assert.dom('[data-test-category="asynchronous"] [data-test-crate-count]').hasText('3,910 crates');
    });

    test('category/:category_id index default sort is recent-downloads', async function(assert) {
        server.create('category', { category: 'Algorithms', crates_cnt: 1 });

        await visit('/categories/algorithms');

        assert.dom('[data-test-category-sort] [data-test-current-order]').hasText('Recent Downloads');
    });
});
