import Service from '@ember/service';
import { tracked } from '@glimmer/tracking';

const DEFAULT_DESCRIPTION = 'cargo is the package manager and crate host for rust';
const DEFAULT_CRATE_DESCRIPTION = 'A package for Rust.';

export default class HeadDataService extends Service {
  @tracked crate;

  get description() {
    return !this.crate ? DEFAULT_DESCRIPTION : this.crate.description || DEFAULT_CRATE_DESCRIPTION;
  }
}
