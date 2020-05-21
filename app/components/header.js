import Component from '@ember/component';
import { on } from '@ember/object/evented';
import { inject as service } from '@ember/service';

import { EKMixin, keyDown, keyPress } from 'ember-keyboard';

export default Component.extend(EKMixin, {
  header: service(),
  router: service(),
  session: service(),

  tagName: '',

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
    search(event) {
      event.preventDefault();

      this.router.transitionTo('search', {
        queryParams: {
          q: this.header.searchValue,
          page: 1,
        },
      });
    },
  },
});
