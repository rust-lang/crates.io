import { helper } from '@ember/component/helper';

import { parseLicense } from '../utils/license';

export default helper(function ([expression]) {
  if (expression) {
    return parseLicense(expression);
  }
});
