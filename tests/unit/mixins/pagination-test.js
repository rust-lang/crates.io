import EmberObject from '@ember/object';
import { module, test } from 'qunit';

import PaginationMixin from '../../../mixins/pagination';

module('Unit | Mixin | pagination', function() {
  // Replace this with your real tests.
  test('it works', function(assert) {
    // eslint-disable-next-line ember/no-new-mixins
    let PaginationObject = EmberObject.extend(PaginationMixin);
    let subject = PaginationObject.create();
    assert.ok(subject);
  });
});
