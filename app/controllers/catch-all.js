import Controller from '@ember/controller';
import { action } from '@ember/object';

export default class CatchAllController extends Controller {
  @action reload(event) {
    event.preventDefault();
    this.model.transition.retry();
  }
}
