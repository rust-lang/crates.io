import { helper } from '@ember/component/helper';

import prettyBytes from 'pretty-bytes';

/**
 * See https://github.com/rust-lang/crates.io/discussions/7177
 * 
 * Here we set fraction digits to 1 because `cargo publish`
 * uses this format (see https://github.com/rust-lang/cargo/blob/master/src/cargo/ops/cargo_package.rs#L168-L171)
*/
export default helper(([bytes], options) => prettyBytes(bytes, {
    ...options,
    binary: true,
    minimumFractionDigits: 1,
    maximumFractionDigits: 1,
}));
