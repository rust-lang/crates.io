import Component from '@ember/component';
import { computed } from '@ember/object';

export default Component.extend({
  tagName: 'button',

  attributeBindings: ['type', 'role', 'disabled'],

  type: computed('tagName', function () {
    return this.tagName === 'button' ? 'button' : null;
  }),

  role: computed('tagName', function () {
    return this.tagName === 'a' ? 'button' : null;
  }),

  disabled: false,

  click() {
    if (!this.disabled) {
      this.toggle();
    }
  },
});
