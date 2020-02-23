import { helper } from '@ember/component/helper';

const numberFormat = new Intl.NumberFormat('en');

export function formatNum(value) {
  return numberFormat.format(value);
}

export default helper(params => formatNum(params[0]));
