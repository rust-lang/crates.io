import Controller from '@ember/controller';
import { action } from '@ember/object';
import { inject as service } from '@ember/service';
import { tracked } from '@glimmer/tracking';

import { task } from 'ember-concurrency';

export default class CrateSettingsController extends Controller {
  @service notifications;
  @service router;

  @tracked reason = '';
  @tracked isConfirmed;

  @action toggleConfirmation() {
    this.isConfirmed = !this.isConfirmed;
  }

  deleteTask = task(async () => {
    let { reason } = this;

    try {
      await this.model.destroyRecord({ adapterOptions: { reason } });
      this.notifications.success(`Crate ${this.model.name} has been successfully deleted.`);
      this.router.transitionTo('index');
    } catch (error) {
      let detail = error.errors?.[0]?.detail;
      if (detail && !detail.startsWith('{')) {
        this.notifications.error(`Failed to delete crate: ${detail}`);
      } else {
        this.notifications.error('Failed to delete crate');
      }
    }
  });
}
