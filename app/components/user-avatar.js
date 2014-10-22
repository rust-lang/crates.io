import Ember from 'ember';

export default Ember.Component.extend({
    size: 'small',
    user: null,
    attributeBindings: ['src', 'width', 'height'],
    tagName: 'img',

    width: function() {
        if (this.get('size') === 'small') {
            return 22;
        } else if (this.get('size') === 'medium-small') {
            return 32;
        } else {
            return 85; // medium
        }
    }.property('size'),

    height: function() {
        return this.get('width');
    }.property('width'),

    src: function() {
        return this.get('user.avatar') + '&s=' + this.get('width');
    }.property('size', 'user'),
});
