import Component from '@ember/component';

export default Component.extend({
  tagName: 'button',

  attributeBindings: ['type', 'role', 'disabled'],

  type: 'button',
  role: null,

  disabled: false,

  click() {
    if (!this.disabled) {
      this.toggle();
    }
  },
});
