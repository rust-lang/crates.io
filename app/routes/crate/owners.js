import Route from '@ember/routing/route';
import { service } from '@ember/service';

export default class OwnersRoute extends Route {
  @service router;

  redirect() {
    let crate = this.modelFor('crate');

    this.router.transitionTo('crate.settings', crate);
  }
}
