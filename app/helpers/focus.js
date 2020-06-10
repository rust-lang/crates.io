import { helper } from '@ember/component/helper';

export function focus([selector]) {
  return function (event) {
    if (document.activeElement === document.body) {
      event.preventDefault();
      document.querySelector(selector).focus();
    }
  };
}

export default helper(focus);
