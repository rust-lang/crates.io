import Component from '@ember/component';
import { action } from '@ember/object';
import { later } from '@ember/runloop';

export default class CrateTomlCopy extends Component {
  tagName = '';

  showSuccess = false;
  showNotification = false;

  toggleClipboardProps(isSuccess) {
    this.setProperties({
      showSuccess: isSuccess,
      showNotification: true,
    });
    later(
      this,
      () => {
        this.set('showNotification', false);
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
