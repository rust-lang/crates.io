import Ember from 'ember';

const { observer } = Ember;

export default Ember.Controller.extend({
    searchController: Ember.inject.controller('search'),

    flashError: null,
    nextFlashError: null,
    search: Ember.computed.oneWay('searchController.q'),

    init() {
        this._super(...arguments);
        Ember.$(document).on('keypress', this.handleKeyPress.bind(this));
        Ember.$(document).on('keydown', this.handleKeyPress.bind(this));
    },

    // Gets the human-readable string for the virtual-key code of the
    // given KeyboardEvent, ev.
    //
    // This function is meant as a polyfill for KeyboardEvent#key,
    // since it is not supported in Trident.  We also test for
    // KeyboardEvent#keyCode because the handleShortcut handler is
    // also registered for the keydown event, because Blink doesn't fire
    // keypress on hitting the Escape key.
    //
    // So I guess you could say things are getting pretty interoperable.
    getVirtualKey(ev) {
        if ('key' in ev && typeof ev.key !== 'undefined') {
            return ev.key;
        }
        const c = ev.charCode || ev.keyCode;
        if (c === 27) {
            return 'Escape';
        }
        return String.fromCharCode(c);
    },

    handleKeyPress(evt) {
        // Don't focus the search field if the user is already using an input element
        if (evt.target.tagName === 'INPUT' || evt.target.tagName === 'TEXTAREA' || evt.target.tagName === 'SELECT') {
            return;
        }
        // Only match plain keys, no modifiers keys
        if (evt.ctrlKey || evt.altKey || evt.metaKey) {
            return;
        }
        if (this.getVirtualKey(evt).toLowerCase() === 's') {
            evt.preventDefault();
            Ember.$('#cargo-desktop-search').focus();
        }
    },

    willDestroy() {
        Ember.$(document).off('keypress');
        Ember.$(document).off('keydown');
    },

    stepFlash() {
        this.set('flashError', this.get('nextFlashError'));
        this.set('nextFlashError', null);
    },

    _scrollToTop() {
        window.scrollTo(0, 0);
    },

    // TODO: remove observer & DOM mutation in controller..
    currentPathChanged: observer('currentPath', function() {
        Ember.run.scheduleOnce('afterRender', this, this._scrollToTop);
    }),

    actions: {
        search() {
            this.transitionToRoute('search', {
                queryParams: {
                    q: this.get('search'),
                    page: 1
                }
            });
        },
    },
});

