import Route from '@ember/routing/route';

export default class OwnersRoute extends Route {
  redirect() {
    let crate = this.modelFor('crate');

    this.transitionTo('crate.settings', crate);
  }
}
