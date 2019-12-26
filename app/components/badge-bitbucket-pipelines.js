import { computed } from '@ember/object';
import { alias } from '@ember/object/computed';
import Component from '@ember/component';

export default Component.extend({
  tagName: '',
  repository: alias('badge.attributes.repository'),

  branch: computed('badge.attributes.branch', function() {
    return encodeURIComponent(this.get('badge.attributes.branch'));
  }),

  text: computed('badge.attributes.branch', function() {
    const branch = this.get('badge.attributes.branch');
    return `Bitbucket Pipelines build status for the ${branch} branch`;
  }),
});
