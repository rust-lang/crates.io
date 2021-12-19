import Route from '@ember/routing/route';
import { inject as service } from '@ember/service';

export default class MeIndexRoute extends Route {
  @service router;

  redirect() {
    this.router.replaceWith('settings');
  }
}
