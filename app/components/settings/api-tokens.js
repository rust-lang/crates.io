import { action } from '@ember/object';
import { notEmpty, filterBy, sort } from '@ember/object/computed';
import { inject as service } from '@ember/service';
import Component from '@glimmer/component';

import { task } from 'ember-concurrency';

export default class ApiTokens extends Component {
  @service store;
  @service notifications;

  tokenSort = ['created_at:desc'];
  @sort('args.tokens', 'tokenSort') sortedTokens;
  @filterBy('args.tokens', 'isNew', true) newTokens;
  @notEmpty('newTokens') disableCreate;

  @action startNewToken() {
    this.store.createRecord('api-token', {
      created_at: new Date(Date.now() + 2000),
    });
  }

  @task(function* (token) {
    try {
      yield token.save();
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
