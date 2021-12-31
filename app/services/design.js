import Service, { inject as service } from '@ember/service';
import { tracked } from '@glimmer/tracking';

import config from '../config/environment';
import * as localStorage from '../utils/local-storage';

const KNOWN_THEMES = new Set(['classic', 'new-design']);

export default class DesignService extends Service {
  @service fastboot;

  @tracked _theme = localStorage.getItem('theme');
  @tracked showToggleButton = config.environment === 'development' || config.environment === 'test';

  get theme() {
    return KNOWN_THEMES.has(this._theme) ? this._theme : 'classic';
  }

  set theme(theme) {
    this._theme = theme;
    localStorage.setItem('theme', theme);
  }
}
