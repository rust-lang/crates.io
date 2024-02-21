import { helper } from '@ember/component/helper';

const IS_SUPPORTED = Boolean(navigator.clipboard?.writeText);

export default helper(() => IS_SUPPORTED);
