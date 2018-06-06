import DS from 'ember-data';

export default DS.RESTAdapter.extend({
    namespace: 'api/v1/me',
    pathForType() {
        return 'crate_owner_invitations';
    },
});
