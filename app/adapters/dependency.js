import ApplicationAdapter from './application';

export default ApplicationAdapter.extend({
    findQuery(store, type, query) {
        if (!query.reverse) {
            return this._super(...arguments);
        }
        delete query.reverse;
        var crate = query.crate;
        delete query.crate;
        return this.ajax(this.urlPrefix() + '/crates/' + crate.get('id') +
                                            '/reverse_dependencies',
                         'GET', { data: query });
    },
});
