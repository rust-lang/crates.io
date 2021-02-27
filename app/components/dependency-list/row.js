import { action } from '@ember/object';
import Component from '@glimmer/component';
import { tracked } from '@glimmer/tracking';

export default class VersionRow extends Component {
  @tracked focused = false;

  @action setFocused(value) {
    this.focused = value;
  }
}
