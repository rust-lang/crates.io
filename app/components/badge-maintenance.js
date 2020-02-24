import Component from '@ember/component';
import { computed } from '@ember/object';
import { alias } from '@ember/object/computed';

export default Component.extend({
  tagName: '',

  escapedStatus: computed('badge', function () {
    return this.get('badge.attributes.status').replace(/-/g, '--');
  }),

  none: computed('badge', function () {
    return this.get('badge.attributes.status') === 'none' || !this.get('badge.attributes.status');
  }),

  status: alias('badge.attributes.status'),

  // eslint-disable-next-line ember/require-return-from-computed
  color: computed('badge', function () {
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

  // eslint-disable-next-line ember/require-return-from-computed
  text: computed('badge', function () {
    switch (this.get('badge.attributes.status')) {
      case 'actively-developed':
        return 'Maintenance intention: Actively developed';
      case 'passively-maintained':
        return 'Maintenance intention: Passively maintained';
      case 'as-is':
        return 'Maintenance intention: As-is';
      case 'experimental':
        return 'Maintenance intention: Experimental';
      case 'looking-for-maintainer':
        return 'Maintenance intention: Looking for maintainer';
      case 'deprecated':
        return 'Maintenance intention: Deprecated';
    }
  }),
});
