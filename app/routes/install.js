import Route from '@ember/routing/route';
import { inject as service } from '@ember/service';

export default Route.extend({
  fastboot: service(),

  redirect() {
    this._redirectTo('https://doc.rust-lang.org/cargo/getting-started/installation.html');
  },

  _redirectTo(url) {
    if (this.fastboot.isFastBoot) {
      let headers = this.fastboot.response.headers;
      headers.set('location', url);
      this.set('fastboot.response.statusCode', 301);
    } else {
      window.location = url;
    }
  },
});
