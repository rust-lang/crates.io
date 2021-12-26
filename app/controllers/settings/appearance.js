import Controller from '@ember/controller';
import { action } from '@ember/object';
import { inject as service } from '@ember/service';

export default class AppearanceSettingsController extends Controller {
  @service design;

  get theme() {
    return this.design.useNewDesign ? 'new-design' : 'classic';
  }

  @action setTheme(theme) {
    this.design.setNewDesign(theme === 'new-design');
  }
}
