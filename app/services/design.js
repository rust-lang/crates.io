import { action } from '@ember/object';
import Service, { inject as service } from '@ember/service';
import { tracked } from '@glimmer/tracking';

import window from 'ember-window-mock';

import config from '../config/environment';

export default class DesignService extends Service {
  @service fastboot;

  @tracked useNewDesign = !this.fastboot.isFastBoot && window.localStorage.getItem('use-new-design') === 'true';
  @tracked showToggleButton = config.environment === 'development';

  constructor() {
    super(...arguments);
    window.toggleDesign = () => this.toggle();
  }

  @action
  toggle() {
    this.useNewDesign = !this.useNewDesign;
    window.localStorage.setItem('use-new-design', String(this.useNewDesign));
  }
}
