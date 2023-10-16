import Service from '@ember/service';

import window from 'ember-window-mock';

export default class RedirectorService extends Service {
  redirectTo(url) {
    window.location = url;
  }
}
