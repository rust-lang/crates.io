import Controller from '@ember/controller';
import { inject as service } from '@ember/service';

import { task } from 'ember-concurrency';

export default class CrateSettingsController extends Controller {
  @service notifications;

  crate = null;
  username = '';

  addOwnerTask = task(async () => {
    const username = this.username;

    try {
      await this.crate.inviteOwner(username);
      if (username.includes(':')) {
        this.notifications.success(`Team ${username} was added as a crate owner`);
        this.crate.owner_team.reload();
      } else {
        this.notifications.success(`An invite has been sent to ${username}`);
      }
    } catch (error) {
      let detail = error.errors?.[0]?.detail;
      if (detail && !detail.startsWith('{')) {
        this.notifications.error(`Error sending invite: ${detail}`);
      } else {
        this.notifications.error('Error sending invite');
      }
    }
  });

  removeOwnerTask = task(async owner => {
    try {
      await this.crate.removeOwner(owner.get('login'));

      if (owner.kind === 'team') {
        this.notifications.success(`Team ${owner.get('display_name')} removed as crate owner`);
        let owner_team = await this.crate.owner_team;
        removeOwner(owner_team, owner);
      } else {
        this.notifications.success(`User ${owner.get('login')} removed as crate owner`);
        let owner_user = await this.crate.owner_user;
        removeOwner(owner_user, owner);
      }
    } catch (error) {
      let subject = owner.kind === 'team' ? `team ${owner.get('display_name')}` : `user ${owner.get('login')}`;
      let message = `Failed to remove the ${subject} as crate owner`;

      let detail = error.errors?.[0]?.detail;
      if (detail && !detail.startsWith('{')) {
        message += `: ${detail}`;
      }

      this.notifications.error(message);
    }
  });
}

function removeOwner(owners, target) {
  let idx = owners.indexOf(target);
  if (idx !== -1) {
    owners.splice(idx, 1);
  }
}
