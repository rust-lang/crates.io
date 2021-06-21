import AuthenticatedRoute from '../-authenticated-route';

export default class SettingsRoute extends AuthenticatedRoute {
  setupController(controller) {
    super.setupController(...arguments);
    let crate = this.modelFor('crate');
    controller.set('crate', crate);
  }
}
