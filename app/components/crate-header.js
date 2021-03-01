import { computed } from '@ember/object';
import { inject as service } from '@ember/service';
import Component from '@glimmer/component';

export default class CrateHeader extends Component {
  @service session;

  @computed('args.crate.owner_user', 'session.currentUser.id')
  get isOwner() {
    return this.args.crate.owner_user.findBy('id', this.session.currentUser?.id);
  }
}
