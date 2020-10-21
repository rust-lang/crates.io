import AuthenticatedRoute from '../-authenticated-route';

export default class MeIndexRoute extends AuthenticatedRoute {
  async model() {
    let { ownedCrates, currentUser: user } = this.session;

    if (!ownedCrates) {
      await this.session.fetchUser();
      ({ ownedCrates } = this.session);
    }

    let apiTokens = await this.store.findAll('api-token');

    return { user, ownedCrates, api_tokens: apiTokens.toArray() };
  }

  setupController(controller) {
    super.setupController(...arguments);

    controller.setProperties({
      emailNotificationsSuccess: false,
      emailNotificationsError: false,
    });
  }
}
