import { service } from '@ember/service';
import Component from '@glimmer/component';

export default class Footer extends Component {
  @service pristineQuery;

  get pristineSupportQuery() {
    let params = this.pristineQuery.paramsFor('support');
    return params;
  }
}
