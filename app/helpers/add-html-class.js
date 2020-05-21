import Helper from '@ember/component/helper';

export default class extends Helper {
  cssClass = null;

  compute(positional, options) {
    this.cssClass = options.class;
    document.documentElement.classList.add(this.cssClass);
  }

  willDestroy() {
    document.documentElement.classList.remove(this.cssClass);
  }
}
