import Ember from 'ember';

export default Ember.Test.registerHelper('hasText', function(app, assert, selector, expected) {
    assert.equal(findWithAssert(selector).text().trim().replace(/\s+/g, ' '), expected);
});
