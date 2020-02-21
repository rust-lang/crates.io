import Component from '@ember/component';
import { computed } from '@ember/object';

import RlDropdownContainer from './rl-dropdown-container';

export default Component.extend({
  classNames: ['rl-dropdown-toggle'],

  tagName: 'button',

  attributeBindings: ['type', 'role', 'disabled'],

  type: computed('tagName', function() {
    return this.tagName === 'button' ? 'button' : null;
  }),

  role: computed('tagName', function() {
    return this.tagName === 'a' ? 'button' : null;
  }),

  dropdownContainer: computed(function() {
    return this.nearestOfType(RlDropdownContainer);
  }),

  action: 'toggleDropdown',

  disabled: false,

  click() {
    if (!this.disabled) {
      this.dropdownContainer.send(this.action);
    }
  },
});
