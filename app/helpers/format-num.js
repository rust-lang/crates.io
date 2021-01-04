import Helper from '@ember/component/helper';

import window from 'ember-window-mock';

function newNumberFormat() {
  try {
    return new Intl.NumberFormat(window.navigator.languages || window.navigator.language);
  } catch {
    return new Intl.NumberFormat('en');
  }
}

export default class FormatNumHelper extends Helper {
  compute([value]) {
    return newNumberFormat().format(value);
  }
}
