import { helper } from '@ember/component/helper';

import prettyBytes from 'pretty-bytes';

/**
 * See https://github.com/rust-lang/crates.io/discussions/7177
 *
 * Here we set fraction digits to 1 because `cargo publish`
 * uses this format (see https://github.com/rust-lang/cargo/blob/0.73.1/src/cargo/ops/cargo_package.rs#L167-L170)
 */
export default helper(([bytes], options) =>
  prettyBytes(bytes, {
    ...options,
    binary: true,
    minimumFractionDigits: 1,
    maximumFractionDigits: 1,
  }),
);
