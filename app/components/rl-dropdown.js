import Component from '@ember/component';
import { alias } from '@ember/object/computed';
import { computed } from '@ember/object';
import $ from 'jquery';

import RlDropdownContainer from './rl-dropdown-container';

export default Component.extend({
    classNames: ['rl-dropdown'],
    classNameBindings: ['isExpanded:open'],

    dropdownContainer: computed(function() {
        return this.nearestOfType(RlDropdownContainer);
    }),

    isExpanded: alias('dropdownContainer.dropdownExpanded'),

    closeOnChildClick: false,

    propagateClicks: true,

    click(event) {
        let closeOnChildClick = this.closeOnChildClick;
        let propagateClicks = this.propagateClicks;
        let $target = $(event.target);
        let $c = this.$();

        if ($target !== $c) {
            if ((closeOnChildClick === true || closeOnChildClick === 'true') && $target.closest($c).length) {
                this.set('isExpanded', false);
            } else if (closeOnChildClick && $target.closest(closeOnChildClick, $c).length) {
                this.set('isExpanded', false);
            }
        }

        if (propagateClicks === false || propagateClicks === 'false') {
            event.stopPropagation();
        }
    },
});
