import ApplicationAdapter from './application';
import Ember from 'ember';

export default ApplicationAdapter.extend({
    pathForType(modelName) {
        var decamelized = Ember.String.underscore(
          Ember.String.decamelize(modelName)
        );
        return Ember.String.pluralize(decamelized);
    }
});
