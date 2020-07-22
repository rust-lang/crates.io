/* global Prism */
import { modifier } from 'ember-modifier';

Prism.plugins.NormalizeWhitespace.setDefaults({
  'left-trim': false,
  'right-trim': true,
  'remove-initial-line-feed': true,
});

export default modifier((element, _, { selector }) => {
  let elements = selector ? element.querySelectorAll(selector) : [element];

  for (let element of elements) {
    Prism.highlightElement(element);
  }
});
