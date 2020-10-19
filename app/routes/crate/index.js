import VersionRoute from './version';

export default class CrateIndexRoute extends VersionRoute {
  controllerName = 'crate.version';
  templateName = 'crate/version';
}
