import Route from '@ember/routing/route';
import { inject as service } from '@ember/service';

export default Route.extend({
  notifications: service(),
  router: service(),
  session: service(),

  async beforeModel(transition) {
    // wait for the `loadUserTask.perform()` of either the `application` route,
    // or the `session.login()` call
    let result = await this.session.loadUserTask.last;

    if (!result.currentUser) {
      this.notifications.error('Please log in to proceed');
      this.session.savedTransition = transition;
      this.router.transitionTo('index');
    }
  },
});
