import Component from '@ember/component';

export default Component.extend({
  classNames: ['rl-dropdown'],
  classNameBindings: ['isExpanded:open'],

  click(event) {
    let closeOnChildClick = 'a:link';
    let $target = event.target;
    let $c = this.element;

    if ($target === $c) {
      return;
    }

    if ($target.closest(closeOnChildClick, $c).length) {
      this.close();
    }
  },
});
