import Component from '@ember/component';
import { computed } from '@ember/object';
import { alias } from '@ember/object/computed';

export default Component.extend({
  tagName: '',

  escapedStatus: computed('badge', function() {
    return this.get('badge.attributes.status').replace(/-/g, '--');
  }),

  none: computed('badge', function() {
    return this.get('badge.attributes.status') === 'none' || !this.get('badge.attributes.status');
  }),

  status: alias('badge.attributes.status'),

  // eslint-disable-next-line ember/require-return-from-computed
  color: computed('badge', function() {
    switch (this.get('badge.attributes.status')) {
      case 'actively-developed':
        return 'brightgreen';
      case 'passively-maintained':
        return 'yellowgreen';
      case 'as-is':
        return 'yellow';
      case 'experimental':
        return 'blue';
      case 'looking-for-maintainer':
        return 'orange';
      case 'deprecated':
        return 'red';
    }
  }),

  text: 'Maintenance intention for this crate',
});
