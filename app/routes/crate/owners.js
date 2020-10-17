import Route from '@ember/routing/route';

export default class OwnersRoute extends Route {
  setupController(controller) {
    super.setupController(...arguments);
    let crate = this.modelFor('crate');
    controller.set('crate', crate);
  }
}
