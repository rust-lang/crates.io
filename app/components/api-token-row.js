import Component from '@ember/component';
import { empty, or } from '@ember/object/computed';

import { task } from 'ember-concurrency';

export default class ApiTokenRow extends Component {
  @empty('api_token.name') emptyName;
  @or('api_token.isSaving', 'emptyName') disableCreate;

  serverError = null;

  didInsertElement() {
    let input = this.element.querySelector('input');
    if (input && input.focus) {
      input.focus();
    }
  }

  @task(function* () {
    try {
      yield this.api_token.save();
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
  })
  saveTokenTask;

  @task(function* () {
    try {
      yield this.api_token.destroyRecord();
    } catch (err) {
      let msg;
      if (err.errors && err.errors[0] && err.errors[0].detail) {
        msg = `An error occurred while revoking this token, ${err.errors[0].detail}`;
      } else {
        msg = 'An unknown error occurred while revoking this token';
      }
      this.set('serverError', msg);
    }
  })
  revokeTokenTask;
}
