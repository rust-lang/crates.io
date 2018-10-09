import { formatCrateSize } from '../../../helpers/format-crate-size';
import { module, test } from 'qunit';

module('Unit | Helper | format-crate-size', function() {
    test('Small crate size formats in kB', function(assert) {
        assert.equal(formatCrateSize(531), '0.53 kB');
    });

    test('Small crate size formats in kB without trailing 0', function(assert) {
        assert.equal(formatCrateSize(90000), '90 kB');
    });

    test('Large crate size formats in MB', function(assert) {
        assert.equal(formatCrateSize(912345), '0.91 MB');
    });

    test('Large crate size formats in MB without trailing 0', function(assert) {
        assert.equal(formatCrateSize(9100000), '9.1 MB');
    });
});
