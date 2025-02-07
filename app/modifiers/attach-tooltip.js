import { autoUpdate, computePosition, flip, offset, shift } from '@floating-ui/dom';
import { modifier } from 'ember-modifier';

export default modifier((floatingElement, positional, { hide, show, side = 'top' } = {}) => {
  let referenceElement = floatingElement.parentElement;

  let cleanup;

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

  function showTooltip() {
    show();
    floatingElement.style.display = 'block';
    cleanup = autoUpdate(referenceElement, floatingElement, update);
  }

  function hideTooltip() {
    hide();
    floatingElement.style.display = '';
    cleanup?.();
  }

  [
    ['mouseenter', showTooltip],
    ['mouseleave', hideTooltip],
    ['focus', showTooltip],
    ['blur', hideTooltip],
  ].forEach(([event, listener]) => {
    referenceElement.addEventListener(event, listener);
  });

  return () => cleanup?.();
});
