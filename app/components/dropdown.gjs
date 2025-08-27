import { fn, hash } from '@ember/helper';
import { action } from '@ember/object';
import Component from '@glimmer/component';
import { tracked } from '@glimmer/tracking';

import onClickOutside from 'ember-click-outside/modifiers/on-click-outside';
import onKey from 'ember-keyboard/modifiers/on-key';

import DropdownContent from 'crates-io/components/dropdown/content';
import DropdownMenu from 'crates-io/components/dropdown/menu';
import DropdownTrigger from 'crates-io/components/dropdown/trigger';

export default class Dropdown extends Component {
  @tracked dropdownExpanded = false;

  @action
  toggleDropdown() {
    this.dropdownExpanded = !this.dropdownExpanded;
  }

  <template>
    <div
      ...attributes
      class='container'
      {{onClickOutside (fn (mut this.dropdownExpanded) false)}}
      {{onKey 'Escape' (fn (mut this.dropdownExpanded) false)}}
    >
      {{yield
        (hash
          Trigger=(component DropdownTrigger toggle=this.toggleDropdown)
          Content=(component DropdownContent isExpanded=this.dropdownExpanded)
          Menu=(component DropdownMenu Content=(component DropdownContent isExpanded=this.dropdownExpanded))
        )
      }}
    </div>
  </template>
}
