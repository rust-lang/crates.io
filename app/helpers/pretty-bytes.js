import { helper } from '@ember/component/helper';

import prettyBytes from 'pretty-bytes';

/**
 * See https://github.com/rust-lang/crates.io/discussions/7177
 */
export default helper(([bytes], options) =>
  prettyBytes(bytes, {
    binary: true,
    ...options,
  }),
);
