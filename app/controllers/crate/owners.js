import Controller from '@ember/controller';

import { task } from 'ember-concurrency';

export default class CrateOwnersController extends Controller {
  crate = null;
  error = false;
  invited = false;
  removed = false;
  username = '';

  @task(function* (event) {
    event.preventDefault();

    this.set('error', false);
    this.set('invited', false);

    const username = this.username;

    if (!username) {
      this.set('error', 'Please enter a username');
      return false;
    }

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
    this.set('removed', false);
    try {
      yield this.crate.removeOwner(owner.get('login'));
      switch (owner.kind) {
        case 'user':
          this.set('removed', `User ${owner.get('login')} removed as crate owner`);
          this.crate.owner_user.removeObject(owner);
          break;
        case 'team':
          this.set('removed', `Team ${owner.get('display_name')} removed as crate owner`);
          this.crate.owner_team.removeObject(owner);
          break;
      }
    } catch (error) {
      if (error.errors) {
        this.set('removed', `Error removing owner: ${error.errors[0].detail}`);
      } else {
        this.set('removed', 'Error removing owner');
      }
    }
  })
  removeOwnerTask;
}
