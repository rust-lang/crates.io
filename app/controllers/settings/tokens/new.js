import Controller from '@ember/controller';
import { action } from '@ember/object';
import { inject as service } from '@ember/service';
import { tracked } from '@glimmer/tracking';

import { task } from 'ember-concurrency';

export default class NewTokenController extends Controller {
  @service notifications;
  @service sentry;
  @service store;
  @service router;

  @tracked name;
  @tracked nameInvalid;

  constructor() {
    super(...arguments);
    this.reset();
  }

  saveTokenTask = task(async () => {
    let { name } = this;
    if (!name) {
      this.nameInvalid = true;
      return;
    }

    let token = this.store.createRecord('api-token', { name });

    try {
      // Save the new API token on the backend
      await token.save();
      // Reset the form
      this.reset();
      // Navigate to the API token list
      this.router.transitionTo('settings.tokens.index');
    } catch (error) {
      // Notify the user
      this.notifications.error('An error has occurred while generating your API token. Please try again later!');
      // Notify the crates.io team
      this.sentry.captureException(error);
      // Notify the developer
      console.error(error);
    }
  });

  reset() {
    this.name = '';
    this.nameInvalid = false;
  }

  @action resetNameValidation() {
    this.nameInvalid = false;
  }
}
