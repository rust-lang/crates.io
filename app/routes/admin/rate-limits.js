import { inject as service } from '@ember/service';

import AuthenticatedRoute from './../-authenticated-route';

export default class RateLimitsAdminRoute extends AuthenticatedRoute {
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
        title: 'This page requires admin authentication',
      });
    } else if (!result.currentUser.admin) {
      this.session.savedTransition = transition;
      this.router.replaceWith('catch-all', {
        transition,
        loginNeeded: false,
        title: 'This page requires admin authentication',
      });
    }
  }
}
