import Component from '@ember/component';
import { computed } from '@ember/object';

export default Component.extend({
  tagName: '',

  version: computed('crate.max_version', function() {
    return this.get('crate.max_version').replace('-', '--');
  }),

  color: computed('crate.max_version', function() {
    if (this.get('crate.max_version')[0] == '0') {
      return 'orange';
    } else {
      return 'blue';
    }
  }),
});
