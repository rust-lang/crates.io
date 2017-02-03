import { test } from 'qunit';
import moduleForAcceptance from 'cargo/tests/helpers/module-for-acceptance';

moduleForAcceptance('Acceptance | categories');

test('listing categories', function(assert) {
    visit('/categories');

    andThen(function() {
        hasText(assert, '.row:eq(0) .desc .info span', '0 crates');

        hasText(assert, '.row:eq(1) .desc .info span', '1 crate');

        hasText(assert, '.row:eq(2) .desc .info span', '3,910 crates');
    });
});
