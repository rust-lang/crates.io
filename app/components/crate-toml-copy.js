import Component from '@ember/component';
import { later } from '@ember/runloop';

export default Component.extend({
  tagName: '',
  copyText: '',
  showSuccess: false,
  showNotification: false,
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
  },
  actions: {
    copySuccess() {
      this.toggleClipboardProps(true);
    },

    copyError() {
      this.toggleClipboardProps(false);
    },
  },
});
