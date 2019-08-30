import Component from '@ember/component';

import DropdownComponentMixin from '../mixins/rl-dropdown-component';

export default Component.extend(DropdownComponentMixin, {
  classNameBindings: ['dropdownExpanded'],
});
