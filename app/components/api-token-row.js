import Component from '@ember/component';
import { empty, or } from '@ember/object/computed';
import { inject as service } from '@ember/service';

import { task } from 'ember-concurrency';

export default class ApiTokenRow extends Component {
  @service notifications;

  @empty('token.name') emptyName;
  @or('token.isSaving', 'emptyName') disableCreate;

  @task(function* () {
    try {
      yield this.token.save();
    } catch (err) {
      let msg;
      if (err.errors && err.errors[0] && err.errors[0].detail) {
        msg = `An error occurred while saving this token, ${err.errors[0].detail}`;
      } else {
        msg = 'An unknown error occurred while saving this token';
      }
      this.notifications.error(msg);
    }
  })
  saveTokenTask;

  @task(function* () {
    try {
      yield this.token.destroyRecord();
    } catch (err) {
      let msg;
      if (err.errors && err.errors[0] && err.errors[0].detail) {
        msg = `An error occurred while revoking this token, ${err.errors[0].detail}`;
      } else {
        msg = 'An unknown error occurred while revoking this token';
      }
      this.notifications.error(msg);
    }
  })
  revokeTokenTask;
}
