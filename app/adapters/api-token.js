import DS from 'ember-data';

export default DS.RESTAdapter.extend({
    namespace: 'me',
    pathForType() {
        return 'tokens';
    }
});
