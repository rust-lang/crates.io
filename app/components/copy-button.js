import { service } from '@ember/service';
import Component from '@glimmer/component';

import { restartableTask } from 'ember-concurrency';

export default class CrateTomlCopy extends Component {
  @service notifications;

  copyTask = restartableTask(async () => {
    let { copyText } = this.args;
    try {
      await navigator.clipboard.writeText(copyText);
      this.notifications.success('Copied to clipboard!');
    } catch {
      this.notifications.error('Copy to clipboard failed!');
    }
  });
}
