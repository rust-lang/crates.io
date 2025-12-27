import { service } from '@ember/service';

import AuthenticatedRoute from '../-authenticated-route';

export default class MeCratesRoute extends AuthenticatedRoute {
  @service router;

  redirect(model, transition) {
    // Redirect to the user's profile page (/users/{username}) with the same query parameters
    let username = this.session.currentUser.login;
    let queryParams = transition.to.queryParams;

    this.router.transitionTo('user', username, { queryParams });
  }
}
