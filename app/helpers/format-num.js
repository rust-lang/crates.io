import { helper } from '@ember/component/helper';

import window from 'ember-window-mock';

let numberFormat;

export function formatNum(value) {
  if (!numberFormat) {
    numberFormat = new Intl.NumberFormat(window.navigator.languages || window.navigator.language);
  }
  return numberFormat.format(value);
}

export default helper(params => formatNum(params[0]));
