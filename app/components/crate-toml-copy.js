import Component from '@ember/component';
import { action } from '@ember/object';
import { later } from '@ember/runloop';
import { tracked } from '@glimmer/tracking';

export default class CrateTomlCopy extends Component {
  tagName = '';

  @tracked showSuccess = false;
  @tracked showNotification = false;

  toggleClipboardProps(isSuccess) {
    this.showSuccess = isSuccess;
    this.showNotification = true;

    later(
      this,
      () => {
        this.showNotification = false;
      },
      2000,
    );
  }

  @action
  copySuccess() {
    this.toggleClipboardProps(true);
  }

  @action
  copyError() {
    this.toggleClipboardProps(false);
  }
}
