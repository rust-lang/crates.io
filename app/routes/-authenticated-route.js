import Route from '@ember/routing/route';
import { inject as service } from '@ember/service';

export default class AuthenticatedRoute extends Route {
  @service router;
  @service session;

  async beforeModel(transition) {
    // wait for the `loadUserTask.perform()` of either the `application` route,
    // or the `session.login()` call
    let result = await this.session.loadUserTask.last;

    if (!result.currentUser) {
      this.session.savedTransition = transition;
      this.router.replaceWith('catch-all', {
        transition,
        loginNeeded: true,
        title: 'This page requires authentication',
      });
    }
  }
}
