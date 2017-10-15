import { test } from 'qunit';
import moduleForAcceptance from 'cargo/tests/helpers/module-for-acceptance';

moduleForAcceptance('Acceptance | categories');

test('listing categories', async function(assert) {
    server.create('category', { category: 'API bindings', crates_cnt: 0 });
    server.create('category', { category: 'Algorithms', crates_cnt: 1 });
    server.create('category', { category: 'Asynchronous', crates_cnt: 3910 });

    await visit('/categories');

    assert.dom('.desc .info span', find('.row').get(0)).hasText('0 crates');
    assert.dom('.desc .info span', find('.row').get(1)).hasText('1 crate');
    assert.dom('.desc .info span', find('.row').get(2)).hasText('3,910 crates');
});

test('category/:category_id index default sort is recent-downloads', async function(assert) {
    server.create('category', { category: 'Algorithms', crates_cnt: 1 });

    await visit('/categories/algorithms');

    assert.dom('div.sort div.dropdown-container a.dropdown').hasText('Recent Downloads');
});
