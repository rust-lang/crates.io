import Helper from '@ember/component/helper';
import { service } from '@ember/service';

import prettyBytes from 'pretty-bytes';

/**
 * See https://github.com/rust-lang/crates.io/discussions/7177
 */
export default class PrettyBytesHelper extends Helper {
  @service intl;

  compute([bytes], options) {
    return prettyBytes(bytes, {
      binary: true,
      locale: this.intl.locale ?? true,
      ...options,
    });
  }
}
