import { computed } from '@ember/object';
import { alias } from '@ember/object/computed';
import Component from '@ember/component';

export default Component.extend({
  tagName: '',
  repository: alias('badge.attributes.repository'),

  branch: computed('badge.attributes.branch', function() {
    return encodeURIComponent(this.get('badge.attributes.branch') || 'master');
  }),

  text: computed('branch', function() {
    return `Circle CI build status for the ${this.branch} branch`;
  }),
});
