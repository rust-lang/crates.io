import { readOnly } from '@ember/object/computed';
import Component from '@ember/component';
import { computed } from '@ember/object';

export default Component.extend({
  size: 'small',
  user: null,
  attributeBindings: ['src', 'width', 'height', 'alt'],
  tagName: 'img',

  width: computed('size', function() {
    if (this.size === 'small') {
      return 22;
    } else if (this.size === 'medium-small') {
      return 32;
    } else {
      return 85; // medium
    }
  }),

  height: readOnly('width'),

  alt: computed('user', function() {
    return `${this.get('user.name')} (${this.get('user.login')})`;
  }),

  src: computed('size', 'user', function() {
    return `${this.get('user.avatar')}&s=${this.width * 2}`;
  }),
});
