import Ember from 'ember';
import FastbootUtilsMixin from 'cargo/mixins/fastboot-utils';
import { module, test } from 'qunit';

module('Unit | Mixin | fastboot utils');

// Replace this with your real tests.
test('it works', function(assert) {
    let FastbootUtilsObject = Ember.Object.extend(FastbootUtilsMixin);
    let subject = FastbootUtilsObject.create();
    assert.ok(subject);
});
