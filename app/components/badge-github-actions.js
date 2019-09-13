import Component from '@ember/component';
import { computed } from '@ember/object';
import { alias } from '@ember/object/computed';

export default Component.extend({
  tagName: 'span',
  classNames: ['badge'],
  repository: alias('badge.attributes.repository'),
  workflow: alias('badge.attributes.workflow'),
  text: computed('badge', function() {
    return `GitHub Actions workflow status for the ${this.workflow} workflow`;
  }),
});
