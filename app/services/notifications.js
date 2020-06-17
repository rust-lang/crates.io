import { A } from '@ember/array';

import NotificationsBaseService from 'ember-cli-notifications/services/notifications';

export default class NotificationsService extends NotificationsBaseService {
  init() {
    super.init(...arguments);

    // workaround for https://github.com/mansona/ember-cli-notifications/issues/277
    this.set('content', A());
  }
}
