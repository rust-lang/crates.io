import { action } from '@ember/object';
import { inject as service } from '@ember/service';
import Component from '@glimmer/component';
import { tracked } from '@glimmer/tracking';

import { task } from 'ember-concurrency';

export default class ApiTokens extends Component {
  @service store;
  @service notifications;

  @tracked newToken;

  get sortedTokens() {
    return this.args.tokens.filter(t => !t.isNew).sort((a, b) => (a.created_at < b.created_at ? 1 : -1));
  }

  @action startNewToken() {
    this.newToken = this.store.createRecord('api-token');
  }

  saveTokenTask = task(async () => {
    let token = this.newToken;

    try {
      await token.save();
      this.args.tokens.unshift(token);
      this.newToken = undefined;
    } catch (error) {
      let msg =
        error.errors && error.errors[0] && error.errors[0].detail
          ? `An error occurred while saving this token, ${error.errors[0].detail}`
          : 'An unknown error occurred while saving this token';

      this.notifications.error(msg);
    }
  });

  revokeTokenTask = task(async token => {
    try {
      await token.destroyRecord();

      let index = this.args.tokens.indexOf(token);
      if (index !== -1) {
        this.args.tokens.splice(index, 1);
      }
    } catch (error) {
      let msg =
        error.errors && error.errors[0] && error.errors[0].detail
          ? `An error occurred while revoking this token, ${error.errors[0].detail}`
          : 'An unknown error occurred while revoking this token';

      this.notifications.error(msg);
    }
  });
}
