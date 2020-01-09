import Controller from '@ember/controller';
import { inject as service } from '@ember/service';
import { oneWay } from '@ember/object/computed';
import { EKMixin, keyDown, keyPress } from 'ember-keyboard';
import { on } from '@ember/object/evented';

export default Controller.extend(EKMixin, {
  flashMessages: service(),
  search: service(),
  searchQuery: oneWay('search.q'),
  session: service(),

  keyboardActivated: true,

  focusSearch: on(keyDown('KeyS'), keyPress('KeyS'), keyDown('shift+KeyS'), function(event) {
    if (event.ctrlKey || event.altKey || event.metaKey) {
      return;
    }

    if (document.activeElement === document.body) {
      event.preventDefault();
      document.querySelector('#cargo-desktop-search').focus();
    }
  }),

  actions: {
    search() {
      this.transitionToRoute('search', {
        queryParams: {
          q: this.searchQuery,
          page: 1,
        },
      });
    },
  },
});
