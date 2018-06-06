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

    propagateClicks: true,

    disabled: false,

    click(event) {
        if (!this.disabled) {
            let propagateClicks = this.propagateClicks;

            this.dropdownContainer.send(this.action);

            if (propagateClicks === false || propagateClicks === 'false') {
                event.stopPropagation();
            }
        }
    },
});
