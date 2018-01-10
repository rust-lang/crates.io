import { test } from 'qunit';
import { visit } from 'ember-native-dom-helpers';
import a11yAudit from 'ember-a11y-testing/test-support/audit';
import moduleForAcceptance from 'cargo/tests/helpers/module-for-acceptance';
import axeConfig from '../axe-config';

moduleForAcceptance('Acceptance | keywords');

test('keyword/:keyword_id is accessible', async function(assert) {
    assert.expect(0);

    server.create('keyword', { id: 'network', keyword: 'network', crates_cnt: 38 });

    await visit('keywords/network');
    await a11yAudit(axeConfig);
});

test('keyword/:keyword_id index default sort is recent-downloads', async function(assert) {
    server.create('keyword', { id: 'network', keyword: 'network', crates_cnt: 38 });

    await visit('/keywords/network');

    assert.dom('[data-test-keyword-sort] [data-test-current-order]').hasText('Recent Downloads');
});
