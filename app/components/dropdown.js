import Component from '@ember/component';
import { action } from '@ember/object';
import { tracked } from '@glimmer/tracking';

export default class Dropdown extends Component {
  tagName = '';

  @tracked dropdownExpanded = false;

  @action
  toggleDropdown() {
    this.dropdownExpanded = !this.dropdownExpanded;
  }
}
