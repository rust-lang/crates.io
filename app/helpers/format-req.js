import { helper } from '@ember/component/helper';

export default helper(function(params) {
  let [req] = params;
  return req === '*' ? '' : req;
});
