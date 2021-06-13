import { action } from '@ember/object';
import Service, { inject as service } from '@ember/service';
import { tracked } from '@glimmer/tracking';
import { lastValue } from 'ember-concurrency';

import window from 'ember-window-mock';

import config from '../config/environment';
import * as localStorage from '../utils/local-storage';

export default class DesignService extends Service {
  @service fastboot;

  @tracked useNewDesign = !this.fastboot.isFastBoot && localStorage.getItem('use-new-design') === 'true';
  @tracked showToggleButton = config.environment === 'development';

  constructor() {
    super(...arguments);
    window.toggleDesign = () => this.toggle();
  }

  @action
  toggle() {
    this.useNewDesign = !this.useNewDesign;
    localStorage.setItem('use-new-design', String(this.useNewDesign));

    document.querySelector('meta[name="theme-color"]').setAttribute(
      "content",
      getComputedStyle(
        document.documentElement
      ).getPropertyValue(
        this.useNewDesign ? '--violet800' : '--green800'
      )
    );
  }
}
