import { action } from '@ember/object';
import { inject as service } from '@ember/service';
import Component from '@glimmer/component';

import copy from 'copy-text-to-clipboard';

export default class CrateTomlCopy extends Component {
  @service notifications;

  @action
  copy() {
    let { copyText } = this.args;

    let success = copy(copyText);
    if (success) {
      this.notifications.success('Copied to clipboard!');
    } else {
      this.notifications.error('Copy to clipboard failed!');
    }
  }
}
