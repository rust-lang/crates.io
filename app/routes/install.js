import Route from '@ember/routing/route';
import { inject as service } from '@ember/service';

export default Route.extend({
  redirector: service(),

  redirect() {
    this.redirector.redirectTo('https://doc.rust-lang.org/cargo/getting-started/installation.html');
  },
});
