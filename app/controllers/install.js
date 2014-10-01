import Ember from 'ember';

export default Ember.Controller.extend({

    linux64: function() {
        return this.link('x86_64-unknown-linux-gnu');
    }.property(),
    linux32: function() {
        return this.link('i686-unknown-linux-gnu');
    }.property(),
    mac64: function() {
        return this.link('x86_64-apple-darwin');
    }.property(),
    mac32: function() {
        return this.link('i686-apple-darwin');
    }.property(),
    win64: function() {
        return this.link('x86_64-w64-mingw32');
    }.property(),
    win32: function() {
        return this.link('i686-w64-mingw32');
    }.property(),

    link: function(target) {
        return 'https://static.rust-lang.org/cargo-dist/cargo-nightly-' +
                    target + '.tar.gz';
    },
});
