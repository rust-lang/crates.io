import Controller from '@ember/controller';
import { computed } from '@ember/object';
import { inject as service } from '@ember/service';

export default Controller.extend({
  session: service(),

  isOwner: computed('model.owner_user', 'session.currentUser.id', function () {
    return this.get('model.owner_user').findBy('id', this.get('session.currentUser.id'));
  }),
});
