import Route from '@ember/routing/route';
import { service } from '@ember/service';

export default class SecurityRoute extends Route {
  @service router;

  redirect() {
    this.router.replaceWith('policies.security');
  }
}
