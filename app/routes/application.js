import { action } from '@ember/object';
import Route from '@ember/routing/route';
import { inject as service } from '@ember/service';

import { shouldPolyfill as shouldPolyfillGetCanonicalLocales } from '@formatjs/intl-getcanonicallocales/should-polyfill';
import { shouldPolyfill as shouldPolyfillLocale } from '@formatjs/intl-locale/should-polyfill';
import { shouldPolyfill as shouldPolyfillNumberFormat } from '@formatjs/intl-numberformat/should-polyfill';
import { shouldPolyfill as shouldPolyfillPluralRules } from '@formatjs/intl-pluralrules/should-polyfill';

export default class ApplicationRoute extends Route {
  @service googleCharts;
  @service notifications;
  @service progress;
  @service session;

  async beforeModel() {
    // trigger the task, but don't wait for the result here
    //
    // we don't need a `catch()` block here because network
    // errors are already dealt with inside of the task
    // and any other errors should end up on Sentry.
    //
    // eslint-disable-next-line ember-concurrency/no-perform-without-catch
    this.session.loadUserTask.perform();

    // start loading the Google Charts JS library already
    // and ignore any errors since we will catch them again
    // anyway when we call `load()` from the `DownloadGraph`
    // component
    this.googleCharts.load().catch(() => {});

    // load `Intl` polyfills if necessary
    let polyfillImports = [];
    if (shouldPolyfillGetCanonicalLocales()) {
      console.debug('Loading Intl.getCanonicalLocales() polyfill…');
      polyfillImports.push(import('@formatjs/intl-getcanonicallocales/polyfill'));
    }
    if (shouldPolyfillLocale()) {
      console.debug('Loading Intl.Locale polyfill…');
      polyfillImports.push(import('@formatjs/intl-locale/polyfill'));
    }
    if (shouldPolyfillPluralRules()) {
      console.debug('Loading Intl.PluralRules polyfill…');
      polyfillImports.push(import('@formatjs/intl-pluralrules/polyfill'));
      polyfillImports.push(import('@formatjs/intl-pluralrules/locale-data/en'));
    }
    if (shouldPolyfillNumberFormat()) {
      console.debug('Loading Intl.NumberFormat polyfill…');
      polyfillImports.push(import('@formatjs/intl-numberformat/polyfill'));
      polyfillImports.push(import('@formatjs/intl-numberformat/locale-data/en'));
    }

    try {
      await Promise.all(polyfillImports);
    } catch {
      let message =
        'We tried to load some polyfill code for your browser, but network issues caused the request to fail. If you notice any issues please try to reload the page.';
      this.notifications.warning(message);
    }
  }

  @action loading(transition) {
    this.progress.handle(transition);
    return true;
  }
}
