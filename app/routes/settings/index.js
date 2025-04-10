import Route from '@ember/routing/route';
import { service } from '@ember/service';

export default class SettingsRoute extends Route {
  @service router;

  redirect() {
    this.router.replaceWith('settings.profile');
  }
}
