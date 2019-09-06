import Component from '@ember/component';

export default Component.extend({
  rendered: '',

  didRender() {
    this._super(...arguments);

    this.element.querySelectorAll('pre > code').forEach(function(node) {
      window.Prism.highlightElement(node);
    });
  },
});
