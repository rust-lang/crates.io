import ApplicationSerializer from './application';

export default class DependencySerializer extends ApplicationSerializer {
  attrs = {
    version: 'version_id',
  };
}
