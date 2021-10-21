import { inject as service } from '@ember/service';
import Component from '@glimmer/component';

export default class CrateHeader extends Component {
  @service session;

  get isOwner() {
    return this.args.crate.owner_user.findBy('id', this.session.currentUser?.id);
  }
}
