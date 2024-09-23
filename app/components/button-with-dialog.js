import { action } from '@ember/object';
import Component from '@glimmer/component';
import { tracked } from '@glimmer/tracking';

export default class ButtonWithConfirmationDialog extends Component {
  @tracked isOpen = false;

  @action toggleDialog(state) {
    this.isOpen = state === undefined ? !this.isOpen : !!state;
  }
}
