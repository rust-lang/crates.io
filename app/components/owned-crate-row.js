import Component from '@ember/component';
import { action, computed } from '@ember/object';
import { alias } from '@ember/object/computed';

export default class OwnedCrateRow extends Component {
  tagName = '';

  @alias('ownedCrate.name') name;

  @computed('ownedCrate.id')
  get controlId() {
    return `${this.ownedCrate.id}-email-notifications`;
  }

  @alias('ownedCrate.email_notifications') emailNotifications;

  @action
  toggleEmailNotifications() {
    this.set('emailNotifications', !this.emailNotifications);
  }
}
