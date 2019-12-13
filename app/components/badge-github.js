import Component from '@ember/component';
import { computed } from '@ember/object';
import { alias } from '@ember/object/computed';

export default Component.extend({
  tagName: 'span',
  classNames: ['badge'],
  repository: alias('badge.attributes.repository'),
  workflow: computed('badge.attributes.workflow', function() {
    return this.get('badge.attributes.workflow') || 'Rust';
  }),
  text: computed('badge', function() {
    return `GitHub build status for the ${this.workflow} workflow`;
  }),
});
