import Component from '@ember/component';

export default Component.extend({
  tagName: '',

  dropdownExpanded: false,

  actions: {
    toggleDropdown() {
      this.toggleProperty('dropdownExpanded');
    },
  },
});
