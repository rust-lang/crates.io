import { helper } from '@ember/component/helper';

import format from 'date-fns/format';

export default helper(function ([date, pattern], options) {
  if (date) {
    return format(date, pattern, { ...options });
  }
});
