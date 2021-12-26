import Service, { inject as service } from '@ember/service';
import { tracked } from '@glimmer/tracking';

import config from '../config/environment';
import * as localStorage from '../utils/local-storage';

export default class DesignService extends Service {
  @service fastboot;

  @tracked useNewDesign = !this.fastboot.isFastBoot && localStorage.getItem('use-new-design') === 'true';
  @tracked showToggleButton = config.environment === 'development' || config.environment === 'test';

  setNewDesign(value) {
    this.useNewDesign = value;
    localStorage.setItem('use-new-design', String(this.useNewDesign));
  }
}
