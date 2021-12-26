import { helper } from '@ember/component/helper';

export default helper(function ([theme]) {
  if (window.document) {
    if (theme) {
      window.document.documentElement.dataset.theme = theme;
    } else {
      delete window.document.documentElement.dataset.theme;
    }
  }
});
