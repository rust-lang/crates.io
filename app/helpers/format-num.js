import Helper from '@ember/component/helper';
import { service } from '@ember/service';

export default class FormatNumHelper extends Helper {
  @service intl;

  compute([value]) {
    return this.intl.formatNumber(value);
  }
}
