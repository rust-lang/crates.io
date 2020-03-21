import { helper } from '@ember/component/helper';

export function or(args) {
  return args.some(Boolean);
}

export default helper(or);
