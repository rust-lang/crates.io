import Helper from '@ember/component/helper';
import { inject as service } from '@ember/service';

export default class FormatNumHelper extends Helper {
  @service intl;

  compute([value]) {
    return this.intl.formatNumber(value);
  }
}
