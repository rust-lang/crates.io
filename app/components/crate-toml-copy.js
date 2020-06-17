import { action } from '@ember/object';
import Component from '@glimmer/component';
import { tracked } from '@glimmer/tracking';

import copy from 'copy-text-to-clipboard';
import { rawTimeout, task } from 'ember-concurrency';

export default class CrateTomlCopy extends Component {
  @tracked showSuccess = false;

  @(task(function* () {
    yield rawTimeout(2000);
  }).restartable())
  showNotificationTask;

  @action
  copy() {
    this.showSuccess = copy(this.args.copyText);
    this.showNotificationTask.perform();
  }
}
