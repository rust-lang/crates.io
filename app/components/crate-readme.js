import Component from '@ember/component';

export default Component.extend({
  rendered: '',
  didRender() {
    this._super(...arguments);
    this.$('pre > code').each(function() {
      window.Prism.highlightElement(this);
    });
    this.scrollToFragment();
  },

  scrollToFragment() {
    if (location.hash) {
      let anchor_id = location.hash.substr(1);
      document.getElementById(anchor_id).scrollIntoView();
    }
  },
});
