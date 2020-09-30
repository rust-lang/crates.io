import Controller from '@ember/controller';
import { action } from '@ember/object';

export default class CatchAllController extends Controller {
  @action search(query) {
    return this.transitionToRoute('search', { queryParams: { q: query } });
  }
}
