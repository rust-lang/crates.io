import { on } from '@ember/modifier';
import { service } from '@ember/service';
import Component from '@glimmer/component';

import { restartableTask } from 'ember-concurrency';
import perform from 'ember-concurrency/helpers/perform';

export default class CrateTomlCopy extends Component {
  <template>
    <button type='button' ...attributes {{on 'click' (perform this.copyTask)}}>
      {{yield}}
    </button>
  </template>
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
