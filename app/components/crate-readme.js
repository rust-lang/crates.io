import Ember from 'ember';

export default Ember.Component.extend({
    rendered: '',
    didRender() {
        this._super(...arguments);
        this.$('pre > code').each(function() {
            window.Prism.highlightElement(this);
        });
    }
});
