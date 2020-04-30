import { helper } from '@ember/component/helper';

import prettyBytes from 'pretty-bytes';

export default helper(([bytes], options) => prettyBytes(bytes, options));
