import Mixin from '@ember/object/mixin';
import { observer } from '@ember/object';
import { on } from '@ember/object/evented';
import { bind, later } from '@ember/runloop';
import $ from 'jquery';

export default Mixin.create({
    init() {
        this._super(...arguments);

        this.set('boundClickoutHandler', bind(this, this.clickoutHandler));
        this.set('boundEscapeHandler', bind(this, this.escapeHandler));
    },

    onOpen() {},
    onClose() {},

    dropdownExpanded: false,

    dropdownToggleSelector: '.rl-dropdown-toggle',

    dropdownSelector: '.rl-dropdown',

    closingEventNamespace: 'rl-dropdown',

    closeOnEscape: true,

    actions: {
        toggleDropdown() {
            this.toggleProperty('dropdownExpanded');

            if (this.dropdownExpanded) {
                this.onOpen();
            } else {
                this.onClose();
            }
        },

        openDropdown() {
            this.set('dropdownExpanded', true);
            this.onOpen();
        },

        closeDropdown() {
            this.set('dropdownExpanded', false);
            this.onClose();
        },
    },

    manageClosingEvents: on(
        'didInsertElement',
        observer('dropdownExpanded', function() {
            const namespace = this.closingEventNamespace;
            const clickEventName = `click.${namespace}`;
            const focusEventName = `focusin.${namespace}`;
            const touchEventName = `touchstart.${namespace}`;
            const escapeEventName = `keydown.${namespace}`;
            const component = this;
            const $document = $(document);

            if (this.dropdownExpanded) {
                /* Add clickout handler with 1ms delay, to allow opening the dropdown
             * by clicking e.g. a checkbox and binding to dropdownExpanded, without
             * having the handler close the dropdown immediately. */
                later(() => {
                    $document.bind(clickEventName, { component }, component.boundClickoutHandler);
                    $document.bind(focusEventName, { component }, component.boundClickoutHandler);
                    $document.bind(touchEventName, { component }, component.boundClickoutHandler);
                }, 1);

                if (this.closeOnEscape) {
                    $document.bind(escapeEventName, { component }, component.boundEscapeHandler);
                }
            } else {
                $document.unbind(clickEventName, component.boundClickoutHandler);
                $document.unbind(focusEventName, component.boundClickoutHandler);
                $document.unbind(touchEventName, component.boundClickoutHandler);
                $document.unbind(escapeEventName, component.boundEscapeHandler);
            }
        }),
    ),

    unbindClosingEvents: on('willDestroyElement', function() {
        const namespace = this.closingEventNamespace;
        const $document = $(document);

        $document.unbind(`click.${namespace}`, this.boundClickoutHandler);
        $document.unbind(`focusin.${namespace}`, this.boundClickoutHandler);
        $document.unbind(`touchstart.${namespace}`, this.boundClickoutHandler);
        $document.unbind(`keydown.${namespace}`, this.boundEscapeHandler);
    }),

    clickoutHandler(event) {
        const { component } = event.data;
        const $c = component.$();
        const $target = $(event.target);

        /* There is an issue when the click triggered a dom change in the
         * dropdown that unloaded the target element. The ancestry of the target
         * can no longer be determined. We can check if html is still an ancestor
         * to determine if this has happened. The safe option then seems to be to
         * not close the dropdown, as occasionaly not closing the dropdown when it
         * should have closed, seems to be less bad for usability than occasionaly
         * closing the dropdown when it should not have closed.
         */
        if (
            component.get('dropdownExpanded') &&
            $target.closest('html').length &&
            !(
                $target.closest($c.find(component.get('dropdownToggleSelector'))).length ||
                $target.closest($c.find(component.get('dropdownSelector'))).length
            )
        ) {
            component.send('closeDropdown');
        }
    },

    escapeHandler(event) {
        if (event.keyCode === 27) {
            event.data.component.send('closeDropdown');
        }
    },
});
