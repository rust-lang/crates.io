import { helper } from '@ember/component/helper';

export default helper(function ([...values]) {
  return values.reduce((a, b) => a + b, 0);
});
