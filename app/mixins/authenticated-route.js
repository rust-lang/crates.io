import Mixin from '@ember/object/mixin';
import { inject as service } from '@ember/service';

// eslint-disable-next-line ember/no-new-mixins
export default Mixin.create({
  flashMessages: service(),
  session: service(),

  beforeModel(transition) {
    return this.session.checkCurrentUser(transition, () => {
      this.flashMessages.queue('Please log in to proceed');
    });
  },
});
