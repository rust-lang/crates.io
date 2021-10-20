import Controller from '@ember/controller';
import { inject as service } from '@ember/service';

import { task } from 'ember-concurrency';

export default class CrateSettingsController extends Controller {
  @service notifications;

  crate = null;
  username = '';

  @task *addOwnerTask() {
    const username = this.username;

    try {
      yield this.crate.inviteOwner(username);
      this.notifications.success(`An invite has been sent to ${username}`);
    } catch (error) {
      if (error.errors) {
        this.notifications.error(`Error sending invite: ${error.errors[0].detail}`);
      } else {
        this.notifications.error('Error sending invite');
      }
    }
  }
  @task *removeOwnerTask(owner) {
    try {
      yield this.crate.removeOwner(owner.get('login'));

      if (owner.kind === 'team') {
        this.notifications.success(`Team ${owner.get('display_name')} removed as crate owner`);
        this.crate.owner_team.removeObject(owner);
      } else {
        this.notifications.success(`User ${owner.get('login')} removed as crate owner`);
        this.crate.owner_user.removeObject(owner);
      }
    } catch (error) {
      let subject = owner.kind === 'team' ? `team ${owner.get('display_name')}` : `user ${owner.get('login')}`;
      let message = `Failed to remove the ${subject} as crate owner`;
      if (error.errors) {
        message += `: ${error.errors[0].detail}`;
      }

      this.notifications.error(message);
    }
  }
}
