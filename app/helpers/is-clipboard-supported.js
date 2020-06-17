import { helper } from '@ember/component/helper';

const IS_SUPPORTED = document.queryCommandSupported && document.queryCommandSupported('copy');

export default helper(() => IS_SUPPORTED);
