import { helper } from '@ember/component/helper';

import formatISO from 'date-fns/formatISO';

export default helper(function ([date], options) {
  if (date) {
    return formatISO(date, { ...options });
  }
});
