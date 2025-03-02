import { action } from '@ember/object';
import Component from '@glimmer/component';
import { tracked } from '@glimmer/tracking';

import { autoUpdate, computePosition, flip, offset, shift } from '@floating-ui/dom';
import { modifier } from 'ember-modifier';

export default class Tooltip extends Component {
  @tracked anchorElement = null;
  @tracked visible = false;

  get containerElement() {
    return document.querySelector('#tooltip-container');
  }

  @action show() {
    this.visible = true;
  }

  @action hide() {
    this.visible = false;
  }

  onInsertAnchor = modifier((anchor, [component]) => {
    component.anchorElement = anchor.parentElement;

    let events = [
      ['mouseenter', component.show],
      ['mouseleave', component.hide],
      ['focus', component.show],
      ['blur', component.hide],
    ];

    for (let [event, listener] of events) {
      component.anchorElement.addEventListener(event, listener);
    }

    return () => {
      for (let [event, listener] of events) {
        component.anchorElement?.removeEventListener(event, listener);
      }
    };
  });

  attachTooltip = modifier((floatingElement, [component], { side = 'top' } = {}) => {
    let referenceElement = component.anchorElement;

    async function update() {
      let middleware = [offset(5), flip(), shift({ padding: 5 })];

      let { x, y } = await computePosition(referenceElement, floatingElement, {
        placement: side,
        middleware,
      });

      Object.assign(floatingElement.style, {
        left: `${x}px`,
        top: `${y}px`,
      });
    }

    let cleanup = autoUpdate(referenceElement, floatingElement, update);

    return () => cleanup();
  });
}
