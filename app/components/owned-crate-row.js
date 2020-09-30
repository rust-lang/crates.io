import Component from '@ember/component';
import { action } from '@ember/object';
import { guidFor } from '@ember/object/internals';

export default class OwnedCrateRow extends Component {
  tagName = '';

  controlId = `${guidFor(this)}-checkbox`;

  @action setEmailNotifications(event) {
    let { checked } = event.target;
    this.ownedCrate.set('email_notifications', checked);
  }
}
