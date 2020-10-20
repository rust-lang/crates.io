import Controller from '@ember/controller';
import { inject as service } from '@ember/service';

import { task } from 'ember-concurrency';

export default class CrateOwnersController extends Controller {
  @service notifications;

  crate = null;
  error = false;
  invited = false;
  username = '';

  @task(function* (event) {
    event.preventDefault();

    this.set('error', false);
    this.set('invited', false);

    const username = this.username;

    try {
      yield this.crate.inviteOwner(username);
      this.set('invited', `An invite has been sent to ${username}`);
    } catch (error) {
      if (error.errors) {
        this.set('error', `Error sending invite: ${error.errors[0].detail}`);
      } else {
        this.set('error', 'Error sending invite');
      }
    }
  })
  addOwnerTask;

  @task(function* (owner) {
    try {
      yield this.crate.removeOwner(owner.get('login'));
      switch (owner.kind) {
        case 'user':
          this.notifications.success(`User ${owner.get('login')} removed as crate owner`);
          this.crate.owner_user.removeObject(owner);
          break;
        case 'team':
          this.notifications.success(`Team ${owner.get('display_name')} removed as crate owner`);
          this.crate.owner_team.removeObject(owner);
          break;
      }
    } catch (error) {
      let subject = owner.kind === 'team' ? `team ${owner.get('display_name')}` : `user ${owner.get('login')}`;
      this.notifications.error(`Failed to remove the ${subject} as crate owner`);
    }
  })
  removeOwnerTask;
}
