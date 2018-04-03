import Component from '@ember/component';

export default Component.extend({
    rendered: '',
    didRender() {
        this._super(...arguments);
        [...this.get('element').querySelectorAll('pre > code')].forEach(function() {
            window.Prism.highlightElement(this);
        });
    }
});
