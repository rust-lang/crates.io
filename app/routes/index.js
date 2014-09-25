import Ember from 'ember';

export default Ember.Route.extend({
  model: function() {
    return {
        num_downloads: 250374858,
        num_packages: 2939959,
        new_packages: [
            {"name": "foo", "version": "0.0.1"},
            {"name": "bar", "version": "0.0.1"},
            {"name": "foobar", "version": "0.0.1"},
            {"name": "baz", "version": "0.0.1"},
            {"name": "test", "version": "0.0.1"},
        ],
        most_downloaded: [
            {"name": "foo", "version": "0.0.1"},
            {"name": "bar", "version": "0.0.1"},
            {"name": "foobar", "version": "0.0.1"},
            {"name": "baz", "version": "0.0.1"},
            {"name": "test", "version": "0.0.1"},
        ],
        just_updated: [
            {"name": "foo", "version": "0.0.1"},
            {"name": "bar", "version": "0.0.1"},
            {"name": "foobar", "version": "0.0.1"},
            {"name": "baz", "version": "0.0.1"},
            {"name": "test", "version": "0.0.1"},
        ],
    };
  }
});

