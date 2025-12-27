import Controller from '@ember/controller';
import { action } from '@ember/object';
import { service } from '@ember/service';
import { tracked } from '@glimmer/tracking';

import { task } from 'ember-concurrency';

export default class CrateSettingsController extends Controller {
  @service notifications;
  @service store;

  crate = null;
  username = '';
  @tracked addOwnerVisible = false;

  /**
   * Tracks whether the trustpub_only checkbox was visible when the page loaded.
   * This prevents the checkbox from disappearing immediately when unchecked
   * if there are no configs - it will only disappear on the next page visit.
   */
  trustpubOnlyCheckboxWasVisible = false;

  get #hasConfigs() {
    return this.githubConfigs?.length > 0 || this.gitlabConfigs?.length > 0;
  }

  get showTrustpubOnlyCheckbox() {
    return this.#hasConfigs || this.crate?.trustpub_only || this.trustpubOnlyCheckboxWasVisible;
  }

  get showTrustpubOnlyWarning() {
    return this.crate?.trustpub_only && !this.#hasConfigs;
  }

  @action showAddOwnerForm() {
    this.addOwnerVisible = true;
    this.username = '';
  }

  addOwnerTask = task(async () => {
    let username = this.username;

    try {
      await this.crate.inviteOwner(username);
      if (username.includes(':')) {
        this.notifications.success(`Team ${username} was added as a crate owner`);
        this.crate.owner_team.reload();
      } else {
        this.notifications.success(`An invite has been sent to ${username}`);
      }
      this.addOwnerVisible = false;
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

  removeConfigTask = task(async config => {
    try {
      await config.destroyRecord();
      this.notifications.success('Trusted Publishing configuration removed successfully');
    } catch (error) {
      let message = 'Failed to remove Trusted Publishing configuration';

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
