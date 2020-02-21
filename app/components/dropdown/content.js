import Component from '@ember/component';
import { alias } from '@ember/object/computed';
import { computed } from '@ember/object';

import Dropdown from '../dropdown';

export default Component.extend({
  classNames: ['rl-dropdown'],
  classNameBindings: ['isExpanded:open'],

  dropdownContainer: computed(function() {
    return this.nearestOfType(Dropdown);
  }),

  isExpanded: alias('dropdownContainer.dropdownExpanded'),

  click(event) {
    let closeOnChildClick = 'a:link';
    let $target = event.target;
    let $c = this.element;

    if ($target === $c) {
      return;
    }

    if ($target.closest(closeOnChildClick, $c).length) {
      this.set('isExpanded', false);
    }
  },
});
