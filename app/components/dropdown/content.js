import Component from '@ember/component';

export default Component.extend({
  classNames: ['rl-dropdown'],
  classNameBindings: ['isExpanded:open'],
});
