import { action } from '@ember/object';
import Component from '@glimmer/component';
import { tracked } from '@glimmer/tracking';

export default class Tooltip extends Component {
  @tracked hidden = true;

  @action hide() {
    this.hidden = true;
  }

  @action show() {
    this.hidden = false;
  }
}
