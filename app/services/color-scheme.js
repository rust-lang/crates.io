import { action } from '@ember/object';
import Service from '@ember/service';
import { tracked } from '@glimmer/tracking';

import { restartableTask, waitForEvent } from 'ember-concurrency';

import * as localStorage from '../utils/local-storage';

const DEFAULT_SCHEME = 'system';
const VALID_SCHEMES = new Set(['light', 'dark', 'system']);
const LS_KEY = 'color-scheme';

export default class DesignService extends Service {
  @tracked _scheme = localStorage.getItem(LS_KEY);
  @tracked resolvedScheme;

  constructor() {
    super(...arguments);
    this.restartWatcherTask();
  }

  get isDark() {
    return this.resolvedScheme === 'dark';
  }

  get scheme() {
    return VALID_SCHEMES.has(this._scheme) ? this._scheme : DEFAULT_SCHEME;
  }

  @action set(scheme) {
    this._scheme = scheme;
    localStorage.setItem(LS_KEY, scheme);
    this.restartWatcherTask();
  }

  restartWatcherTask() {
    this.watcherTask.perform().catch(() => {
      // Ignore Promise rejections. This shouldn't be able to fail, and task cancellations are expected.
    });
  }

  /**
   * This task watches for changes in the system color scheme and updates the `resolvedScheme` property accordingly.
   */
  watcherTask = restartableTask(async () => {
    let mediaQueryList = window.matchMedia('(prefers-color-scheme: dark)');
    // eslint-disable-next-line no-constant-condition
    while (true) {
      let scheme = this.scheme;
      if (scheme === 'system') {
        scheme = mediaQueryList.matches ? 'dark' : 'light';
      }

      if (this.resolvedScheme !== scheme) {
        this.resolvedScheme = scheme;
      }

      await waitForEvent(mediaQueryList, 'change');
    }
  });
}
