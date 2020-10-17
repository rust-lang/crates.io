import ApplicationAdapter from './application';

export default class DependencyAdapter extends ApplicationAdapter {
  query(store, type, query) {
    if (!query.reverse) {
      return super.query(...arguments);
    }
    delete query.reverse;
    let { crate } = query;
    delete query.crate;
    return this.ajax(`/${this.urlPrefix()}/crates/${crate.get('id')}/reverse_dependencies`, 'GET', { data: query });
  }
}
