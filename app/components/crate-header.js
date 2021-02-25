import { inject as service } from '@ember/service';
import Component from '@glimmer/component';

export default class CrateHeader extends Component {
  @service router;
  @service session;
}
