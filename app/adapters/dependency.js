import ApplicationAdapter from './application';

export default ApplicationAdapter.extend({
    query(store, type, query) {
        if (!query.reverse) {
            return this._super(...arguments);
        }
        delete query.reverse;
        let { crate } = query;
        delete query.crate;
        return this.ajax(`/${this.urlPrefix()}/crates/${crate.get('id')}/reverse_dependencies`, 'GET', { data: query });
    },
});
