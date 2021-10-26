import Controller from '@ember/controller';
import { inject as service } from '@ember/service';

export default class ApplicationController extends Controller {
  @service design;
  @service progress;
  @service router;

  get isIndex() {
    return this.router.currentRouteName === 'index';
  }
}
