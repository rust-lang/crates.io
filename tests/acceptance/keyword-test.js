import { test } from 'qunit';
import moduleForAcceptance from 'cargo/tests/helpers/module-for-acceptance';
import hasText from 'cargo/tests/helpers/has-text';

moduleForAcceptance('Acceptance | keywords');

test('keyword/:keyword_id index default sort is recent-downloads', async function(assert) {
    server.create('keyword', { id: 'network', keyword: 'network', crates_cnt: 38 });

    await visit('/keywords/network');

    const $sort = findWithAssert('div.sort div.dropdown-container a.dropdown');
    hasText(assert, $sort, 'Recent Downloads');
});
