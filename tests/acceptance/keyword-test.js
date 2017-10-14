import { test } from 'qunit';
import moduleForAcceptance from 'cargo/tests/helpers/module-for-acceptance';

moduleForAcceptance('Acceptance | keywords');

test('keyword/:keyword_id index default sort is recent-downloads', async function(assert) {
    server.create('keyword', { id: 'network', keyword: 'network', crates_cnt: 38 });

    await visit('/keywords/network');

    assert.dom('div.sort div.dropdown-container a.dropdown').hasText('Recent Downloads');
});
