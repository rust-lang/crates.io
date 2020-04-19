import Mixin from '@ember/object/mixin';
import { inject as service } from '@ember/service';

// eslint-disable-next-line ember/no-new-mixins
export default Mixin.create({
  flashMessages: service(),
  router: service(),
  session: service(),

  async beforeModel(transition) {
    // wait for the `loadUserTask.perform()` of either the `application` route,
    // or the `session.login()` call
    let result = await this.session.loadUserTask.last;

    if (!result.currentUser) {
      this.flashMessages.queue('Please log in to proceed');
      this.session.savedTransition = transition;
      this.router.transitionTo('index');
    }
  },
});
