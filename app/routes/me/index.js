import Route from '@ember/routing/route';

import AuthenticatedRoute from '../../mixins/authenticated-route';

export default Route.extend(AuthenticatedRoute, {
  actions: {
    willTransition: function () {
      this.controller
        .setProperties({
          emailNotificationsSuccess: false,
          emailNotificationsError: false,
        })
        .clear();
    },
  },
  async model() {
    let { ownedCrates, currentUser: user } = this.session;

    if (ownedCrates.length === 0) {
      await this.session.fetchUser();
      ({ ownedCrates } = this.session);
    }

    let apiTokens = this.store.findAll('api-token');

    return { user, ownedCrates, api_tokens: apiTokens };
  },
});
