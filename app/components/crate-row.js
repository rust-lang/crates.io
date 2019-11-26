import Component from '@ember/component';
import { computed } from '@ember/object';

export default Component.extend({
  classNames: ['crate', 'row'],
  crateTomlText: computed('crate.name', 'max_version', function() {
    return `${this.get('crate.name')} = "${this.get('crate.max_version')}"`;
  }),

  'data-test-crate-row': true,
});
