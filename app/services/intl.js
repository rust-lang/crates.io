import Service from '@ember/service';

import window from 'ember-window-mock';

function newNumberFormat() {
  try {
    return new Intl.NumberFormat(window.navigator.languages || window.navigator.language);
  } catch {
    return new Intl.NumberFormat('en');
  }
}

export default class IntlService extends Service {
  formatNumber(value) {
    return newNumberFormat().format(value);
  }
}
