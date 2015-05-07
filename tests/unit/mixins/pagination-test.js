import Ember from 'ember';
import PaginationMixin from 'cargo/mixins/pagination';
import { module } from 'qunit';
import { test } from 'ember-qunit';

module('PaginationMixin');

// Replace this with your real tests.
test('it works', function(assert) {
  var PaginationObject = Ember.Object.extend(PaginationMixin);
  var subject = PaginationObject.create();
  assert.ok(subject);
});
