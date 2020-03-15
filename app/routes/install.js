import Route from '@ember/routing/route';
import { inject as service } from '@ember/service';

export default class InstallRoute extends Route {
  @service redirector;

  redirect() {
    this.redirector.redirectTo('https://doc.rust-lang.org/cargo/getting-started/installation.html');
  }
}
