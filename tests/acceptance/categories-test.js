import { test } from 'qunit';
import moduleForAcceptance from 'cargo/tests/helpers/module-for-acceptance';
import hasText from 'cargo/tests/helpers/has-text';

moduleForAcceptance('Acceptance | categories');

test('listing categories', async function(assert) {
    await visit('/categories');

    hasText(assert, '.row:eq(0) .desc .info span', '0 crates');
    hasText(assert, '.row:eq(1) .desc .info span', '1 crate');
    hasText(assert, '.row:eq(2) .desc .info span', '3,910 crates');
});
