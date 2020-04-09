import Controller from '@ember/controller';
import { oneWay } from '@ember/object/computed';
import { on } from '@ember/object/evented';
import { inject as service } from '@ember/service';

import { EKMixin, keyDown, keyPress } from 'ember-keyboard';

export default Controller.extend(EKMixin, {
  flashMessages: service(),
  search: service(),
  searchQuery: oneWay('search.q'),
  session: service(),

  keyboardActivated: true,

  focusSearch: on(keyDown('KeyS'), keyPress('KeyS'), keyDown('shift+KeyS'), function (event) {
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
