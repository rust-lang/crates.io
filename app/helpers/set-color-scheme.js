import { helper } from '@ember/component/helper';

export default helper(function ([mode]) {
  if (mode) {
    window.document.documentElement.dataset.colorScheme = mode;
  } else {
    delete window.document.documentElement.dataset.colorScheme;
  }
});
