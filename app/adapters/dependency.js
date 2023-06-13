import ApplicationAdapter from './application';

export default class DependencyAdapter extends ApplicationAdapter {
  query(store, type, query) {
    let { crate, reverse, ...data } = query;

    return reverse
      ? this.ajax(`/${this.urlPrefix()}/crates/${crate.id}/reverse_dependencies`, 'GET', { data })
      : super.query(...arguments);
  }
}
