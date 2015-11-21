import Ember from 'ember';
import PaginationMixin from '../../../mixins/pagination';
import { module, test } from 'qunit';

module('Unit | Mixin | pagination');

// Replace this with your real tests.
test('it works', function(assert) {
  let PaginationObject = Ember.Object.extend(PaginationMixin);
  let subject = PaginationObject.create();
  assert.ok(subject);
});
