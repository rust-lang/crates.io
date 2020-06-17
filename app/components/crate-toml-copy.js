import Component from '@glimmer/component';
import { tracked } from '@glimmer/tracking';

import { rawTimeout, task } from 'ember-concurrency';

export default class CrateTomlCopy extends Component {
  @tracked showSuccess = false;
  @tracked showNotification = false;

  @(task(function* (isSuccess) {
    this.showSuccess = isSuccess;
    this.showNotification = true;
    yield rawTimeout(2000);
    this.showNotification = false;
  }).restartable())
  showNotificationTask;
}
