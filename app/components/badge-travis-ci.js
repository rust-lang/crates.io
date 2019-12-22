import Component from '@ember/component';
import { computed } from '@ember/object';
import { alias } from '@ember/object/computed';

export default Component.extend({
  tagName: '',
  repository: alias('badge.attributes.repository'),

  branch: computed('badge.attributes.branch', function() {
    return this.get('badge.attributes.branch') || 'master';
  }),

  text: computed('branch', function() {
    return `Travis CI build status for the ${this.branch} branch`;
  }),
});
