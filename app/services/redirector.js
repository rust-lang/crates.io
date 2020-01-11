import Service, { inject as service } from '@ember/service';
import window from 'ember-window-mock';

export default Service.extend({
  fastboot: service(),

  redirectTo(url) {
    if (this.fastboot.isFastBoot) {
      let headers = this.fastboot.response.headers;
      headers.set('location', url);
      this.set('fastboot.response.statusCode', 301);
    } else {
      window.location = url;
    }
  },
});
