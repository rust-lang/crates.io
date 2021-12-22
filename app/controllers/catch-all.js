import Controller from '@ember/controller';
import { action } from '@ember/object';

export default class CatchAllController extends Controller {
  @action reload() {
    this.model.transition.retry();
  }

  @action back() {
    history.back();
  }
}
