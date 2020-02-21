import { helper } from '@ember/component/helper';

import formatDistanceToNow from 'date-fns/formatDistanceToNow';

export default helper(function ([date], options) {
  if (date) {
    return formatDistanceToNow(date, { ...options });
  }
});
