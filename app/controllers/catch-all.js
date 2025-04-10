import Controller from '@ember/controller';
import { action } from '@ember/object';
import { service } from '@ember/service';

export default class CatchAllController extends Controller {
  @service router;
  @service session;

  @action reload() {
    this.router.replaceWith(this.router.currentURL);
  }

  @action back() {
    history.back();
  }
}
