/* global Prism */
import { modifier } from 'ember-modifier';

export default modifier((element, _, { selector }) => {
  let elements = selector ? element.querySelectorAll(selector) : [element];

  for (let element of elements) {
    Prism.highlightElement(element);
  }
});
