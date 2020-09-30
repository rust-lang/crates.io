import { action } from '@ember/object';
import { guidFor } from '@ember/object/internals';
import Component from '@glimmer/component';

export default class OwnedCrateRow extends Component {
  controlId = `${guidFor(this)}-checkbox`;

  @action setEmailNotifications(event) {
    let { checked } = event.target;
    this.args.ownedCrate.set('email_notifications', checked);
  }
}
