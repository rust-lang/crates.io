import Component from '@ember/component';
import { action, computed } from '@ember/object';

export default class OwnedCrateRow extends Component {
  tagName = '';

  @computed('ownedCrate.id')
  get controlId() {
    return `${this.ownedCrate.id}-email-notifications`;
  }

  @action setEmailNotifications(event) {
    let { checked } = event.target;
    this.ownedCrate.set('email_notifications', checked);
  }
}
