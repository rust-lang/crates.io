import Component from '@ember/component';
import { EKMixin, EKOnInsertMixin, keyDown } from 'ember-keyboard';
import { on } from '@ember/object/evented';

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
