import Component from '@ember/component';
import { on } from '@ember/object/evented';

import { EKMixin, EKOnInsertMixin, keyDown } from 'ember-keyboard';

export default Component.extend(EKMixin, EKOnInsertMixin, {
  tagName: '',

  dropdownExpanded: false,

  onEscape: on(keyDown('Escape'), function () {
    this.set('dropdownExpanded', false);
  }),

  actions: {
    toggleDropdown() {
      this.toggleProperty('dropdownExpanded');
    },
  },
});
