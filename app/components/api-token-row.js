import Component from '@ember/component';
import { inject as service } from '@ember/service';
import { empty, or } from '@ember/object/computed';

export default Component.extend({
  emptyName: empty('api_token.name'),
  disableCreate: or('api_token.isSaving', 'emptyName'),
  serverError: null,
  session: service(),
  store: service(),

  didInsertElement() {
    let input = this.element.querySelector('input');
    if (input && input.focus) {
      input.focus();
    }
  },

  actions: {
    async saveToken() {
      try {
        await this.api_token.save();
        this.set('session.currentUser.has_tokens', true);
        this.set('serverError', null);
      } catch (err) {
        let msg;
        if (err.errors && err.errors[0] && err.errors[0].detail) {
          msg = `An error occurred while saving this token, ${err.errors[0].detail}`;
        } else {
          msg = 'An unknown error occurred while saving this token';
        }
        this.set('serverError', msg);
      }
    },

    async revokeToken() {
      try {
        // To avoid error on destroy we need to set before destroying of api-token
        // that's why we need to set length of api-tokens to 1 in check
        if ((await this.store.query('api-token', {})).length == 1) {
          this.set('session.currentUser.has_tokens', false);
        }
        await this.api_token.destroyRecord();
      } catch (err) {
        let msg;
        if (err.errors && err.errors[0] && err.errors[0].detail) {
          msg = `An error occurred while revoking this token, ${err.errors[0].detail}`;
        } else {
          msg = 'An unknown error occurred while revoking this token';
        }
        this.set('serverError', msg);
      }
    },
  },
});
