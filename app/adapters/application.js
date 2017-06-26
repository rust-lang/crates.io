import DS from 'ember-data';
import AdapterFetch from 'ember-fetch/mixins/adapter-fetch';

const { RESTAdapter } = DS;

export default RESTAdapter.extend(AdapterFetch, {
    namespace: 'api/v1',
});
