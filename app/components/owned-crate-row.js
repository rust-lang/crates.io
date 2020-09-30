import { action } from '@ember/object';
import Component from '@glimmer/component';

export default class OwnedCrateRow extends Component {
  @action setEmailNotifications(event) {
    let { checked } = event.target;
    this.args.ownedCrate.set('email_notifications', checked);
  }
}
