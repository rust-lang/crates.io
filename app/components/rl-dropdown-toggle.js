import Component from '@ember/component';
import { computed } from '@ember/object';

import RlDropdownContainer from './rl-dropdown-container';

export default Component.extend({
    classNames: ['rl-dropdown-toggle'],

    tagName: 'button',

    attributeBindings: ['type', 'role', 'disabled'],

    type: computed('tagName', function() {
        return this.get('tagName') === 'button' ? 'button' : null;
    }),

    role: computed('tagName', function() {
        return this.get('tagName') === 'a' ? 'button' : null;
    }),

    dropdownContainer: computed(function() {
        return this.nearestOfType(RlDropdownContainer);
    }),

    action: 'toggleDropdown',

    propagateClicks: true,

    disabled: false,

    click(event) {
        if (!this.get('disabled')) {
            let propagateClicks = this.get('propagateClicks');

            this.get('dropdownContainer').send(this.get('action'));

            if (propagateClicks === false || propagateClicks === 'false') {
                event.stopPropagation();
            }
        }
    }
});
