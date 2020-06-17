import { action } from '@ember/object';
import Component from '@glimmer/component';
import { tracked } from '@glimmer/tracking';

export default class Dropdown extends Component {
  @tracked dropdownExpanded = false;

  @action
  toggleDropdown() {
    this.dropdownExpanded = !this.dropdownExpanded;
  }
}
