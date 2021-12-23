import Controller from '@ember/controller';
import { action } from '@ember/object';
import { inject as service } from '@ember/service';

export default class CatchAllController extends Controller {
  @service session;

  @action reload() {
    this.model.transition.retry();
  }

  @action back() {
    history.back();
  }
}
