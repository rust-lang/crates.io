import Component from '@ember/component';

export default Component.extend({
    rendered: '',
    didRender() {
        this._super(...arguments);
        this.$('pre > code').each(function() {
            window.Prism.highlightElement(this);
        });
    },
});
