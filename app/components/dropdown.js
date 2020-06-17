import Component from '@ember/component';
import { action } from '@ember/object';

export default class Dropdown extends Component {
  tagName = '';

  dropdownExpanded = false;

  @action
  toggleDropdown() {
    this.toggleProperty('dropdownExpanded');
  }
}
