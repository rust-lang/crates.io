import Controller from '@ember/controller';
import { action } from '@ember/object';
import { inject as service } from '@ember/service';

import { alias } from 'macro-decorators';

export default class AppearanceSettingsController extends Controller {
  @service design;

  @alias('design.theme') theme;

  @action setTheme(theme) {
    this.theme = theme;
  }
}
