import { helper } from '@ember/component/helper';

import prettyBytes from 'pretty-bytes';

/**
 * See https://github.com/rust-lang/crates.io/discussions/7177
 * 
 * Here set {minimum,maximum}FractionDigits to 1 because
 * `cargo publish` uses this format
*/
export default helper(([bytes], options) => prettyBytes(bytes, {
    ...options,
    binary: true,
    minimumFractionDigits: 1,
    maximumFractionDigits: 1,
}));
