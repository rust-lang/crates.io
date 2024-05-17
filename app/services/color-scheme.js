import { action } from '@ember/object';
import Service from '@ember/service';
import { tracked } from '@glimmer/tracking';

import * as localStorage from '../utils/local-storage';

const DEFAULT_SCHEME = 'light';
const VALID_SCHEMES = new Set(['light', 'dark', 'system']);
const LS_KEY = 'color-scheme';

export default class DesignService extends Service {
  @tracked _scheme = localStorage.getItem(LS_KEY);

  get scheme() {
    return VALID_SCHEMES.has(this._scheme) ? this._scheme : DEFAULT_SCHEME;
  }

  @action set(scheme) {
    this._scheme = scheme;
    localStorage.setItem(LS_KEY, scheme);
  }
}
