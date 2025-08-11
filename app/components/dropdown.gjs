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

<div
  ...attributes
  class="container"
  {{on-click-outside (fn (mut this.dropdownExpanded) false)}}
  {{on-key 'Escape' (fn (mut this.dropdownExpanded) false)}}
>
  {{yield (hash
    Trigger=(component "dropdown/trigger" toggle=this.toggleDropdown)
    Content=(component "dropdown/content" isExpanded=this.dropdownExpanded)
    Menu=(component "dropdown/menu" Content=(component "dropdown/content" isExpanded=this.dropdownExpanded))
  )}}
</div>