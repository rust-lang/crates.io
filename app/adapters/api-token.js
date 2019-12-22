import RESTAdapter from '@ember-data/adapter/rest';

export default RESTAdapter.extend({
  namespace: 'api/v1/me',
  pathForType() {
    return 'tokens';
  },
  createRecord(store, type, snapshot) {
    let data = {};
    let serializer = store.serializerFor(type.modelName);
    let url = this.buildURL(type.modelName, null, snapshot, 'createRecord');

    serializer.serializeIntoHash(data, type, snapshot, { includeId: true });

    return this.ajax(url, 'PUT', { data });
  },
});
