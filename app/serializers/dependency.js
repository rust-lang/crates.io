import ApplicationSerializer from './application';

export default class DependencySerializer extends ApplicationSerializer {
  attrs = {
    crate: 'crate_id',
    version: 'version_id',
  };
}
