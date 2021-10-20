import { action } from '@ember/object';
import { inject as service } from '@ember/service';
import Component from '@glimmer/component';
import { tracked } from '@glimmer/tracking';

import { task } from 'ember-concurrency';
import { sortBy } from 'macro-decorators';

export default class ApiTokens extends Component {
  @service store;
  @service notifications;

  @tracked newToken;

  @sortBy('args.tokens', 'created_at', false) sortedTokens;

  @action startNewToken() {
    this.newToken = this.store.createRecord('api-token');
  }

  @task(function* () {
    let token = this.newToken;

    try {
      yield token.save();
      this.args.tokens.unshiftObject(token);
      this.newToken = undefined;
    } catch (error) {
      let msg =
        error.errors && error.errors[0] && error.errors[0].detail
          ? `An error occurred while saving this token, ${error.errors[0].detail}`
          : 'An unknown error occurred while saving this token';

      this.notifications.error(msg);
    }
  })
  saveTokenTask;

  @task(function* (token) {
    try {
      yield token.destroyRecord();
      this.args.tokens.removeObject(token);
    } catch (error) {
      let msg =
        error.errors && error.errors[0] && error.errors[0].detail
          ? `An error occurred while revoking this token, ${error.errors[0].detail}`
          : 'An unknown error occurred while revoking this token';

      this.notifications.error(msg);
    }
  })
  revokeTokenTask;
}
