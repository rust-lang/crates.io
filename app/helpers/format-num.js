import { helper } from '@ember/component/helper';

import window from 'ember-window-mock';

function newNumberFormat() {
  try {
    return new Intl.NumberFormat(window.navigator.languages || window.navigator.language);
  } catch (error) {
    return new Intl.NumberFormat('en');
  }
}

export function formatNum(value) {
  return newNumberFormat().format(value);
}

export default helper(params => formatNum(params[0]));
