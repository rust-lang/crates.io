import { inject as service } from '@ember/service';
import Component from '@glimmer/component';

import { task } from 'ember-concurrency';

export default class ApiTokenRow extends Component {
  @service notifications;

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
