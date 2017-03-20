import Ember from 'ember';
import RlDropdownContainer from './rl-dropdown-container';

export default Ember.Component.extend({
    classNames: ['rl-dropdown-toggle'],

    tagName: 'button',

    attributeBindings: ['type', 'role', 'disabled'],

    type: Ember.computed('tagName', function() {
        return this.get('tagName') === 'button' ? 'button' : null;
    }),

    role: Ember.computed('tagName', function() {
        return this.get('tagName') === 'a' ? 'button' : null;
    }),

    dropdownContainer: Ember.computed(function() {
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
