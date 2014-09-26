import Ember from 'ember';
import PaginationMixin from 'cargo/mixins/pagination';

module('PaginationMixin');

// Replace this with your real tests.
test('it works', function() {
  var PaginationObject = Ember.Object.extend(PaginationMixin);
  var subject = PaginationObject.create();
  ok(subject);
});
