import { action } from '@ember/object';
import Component from '@glimmer/component';
import { tracked } from '@glimmer/tracking';

import copy from 'copy-text-to-clipboard';
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

  @action
  copy() {
    let isSuccess = copy(this.args.copyText);
    this.showNotificationTask.perform(isSuccess);
  }
}
