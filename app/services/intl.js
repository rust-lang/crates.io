import Service from '@ember/service';

export default class IntlService extends Service {
  // `undefined` means "use the default language of the browser"
  locale = undefined;

  formatNumber(value, options) {
    return Number(value).toLocaleString(this.locale, options);
  }
}
