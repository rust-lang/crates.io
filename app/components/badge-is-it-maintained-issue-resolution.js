import Component from '@ember/component';
import { computed } from '@ember/object';
import { alias } from '@ember/object/computed';

export default Component.extend({
  tagName: '',
  repository: alias('badge.attributes.repository'),

  text: computed('badge', function() {
    return `Is It Maintained average time to resolve an issue`;
  }),
});
