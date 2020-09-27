import Controller from '@ember/controller';
import { computed } from '@ember/object';
import { inject as service } from '@ember/service';

export default class CrateVersionsController extends Controller {
  @service session;

  @computed('model.owner_user', 'session.currentUser.id')
  get isOwner() {
    return this.get('model.owner_user').findBy('id', this.get('session.currentUser.id'));
  }
}
