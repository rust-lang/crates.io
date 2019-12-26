import { helper } from '@ember/component/helper';

export function eq([a, b]) {
  return a === b;
}

export default helper(eq);
