import { helper } from '@ember/component/helper';
import { htmlSafe as markAsSafe } from '@ember/template';

export function htmlSafe([content] /*, hash*/) {
  return markAsSafe(content);
}

export default helper(htmlSafe);
