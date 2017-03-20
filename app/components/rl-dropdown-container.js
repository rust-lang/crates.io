import Ember from 'ember';
import DropdownComponentMixin from '../mixins/rl-dropdown-component';

export default Ember.Component.extend(DropdownComponentMixin, {
  classNameBindings: ['dropdownExpanded']
});
