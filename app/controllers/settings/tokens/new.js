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
  @tracked scopes;
  @tracked scopesInvalid;

  ENDPOINT_SCOPES = [
    { id: 'change-owners', description: 'Invite new crate owners or remove existing ones' },
    { id: 'publish-new', description: 'Publish new crates' },
    { id: 'publish-update', description: 'Publish new versions of existing crates' },
    { id: 'yank', description: 'Yank and unyank crate versions' },
  ];

  constructor() {
    super(...arguments);
    this.reset();
  }

  @action isScopeSelected(id) {
    return this.scopes.includes(id);
  }

  saveTokenTask = task(async () => {
    if (!this.validate()) return;
    let { name, scopes } = this;

    let token = this.store.createRecord('api-token', { name, endpoint_scopes: scopes });

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
    this.scopes = [];
    this.scopesInvalid = false;
  }

  validate() {
    this.nameInvalid = !this.name;
    this.scopesInvalid = this.scopes.length === 0;

    return !this.nameInvalid && !this.scopesInvalid;
  }

  @action resetNameValidation() {
    this.nameInvalid = false;
  }

  @action toggleScope(id) {
    this.scopes = this.scopes.includes(id) ? this.scopes.filter(it => it !== id) : [...this.scopes, id];
    this.scopesInvalid = false;
  }
}
