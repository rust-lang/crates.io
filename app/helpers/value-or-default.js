import Ember from 'ember';

export function valueOrDefault(value, default_value) {
    return value ? value : default_value;
}

export default Ember.Helper.helper(params => valueOrDefault(params[0], params[1]));
