import Controller from '@ember/controller';
import { inject as service } from '@ember/service';
import { oneWay } from '@ember/object/computed';
import { EKMixin, keyDown, keyPress } from 'ember-keyboard';
import { on } from '@ember/object/evented';

export default Controller.extend(EKMixin, {
    search: service(),
    searchQuery: oneWay('search.q'),

    keyboardActivated: true,
    focusSearch: on(keyDown('KeyS'), keyPress('KeyS'), function(event) {
        if (event.ctrlKey || event.altKey || event.metaKey) {
            return;
        }
        event.preventDefault();
        let searchInput = document.querySelector('#cargo-desktop-search');
        if (searchInput) {
            searchInput.focus();
        }
    }),

    actions: {
        search() {
            this.transitionToRoute('search', {
                queryParams: {
                    q: this.get('searchQuery'),
                    page: 1
                }
            });
        },
    },
});

