import { setTesting } from '@ember/debug';
import Service from '@ember/service';

export default class extends Service {
  Service = Service;

  setTesting(value) {
    // This indirection is needed for playwright to be able to use the `setTesting()` fn of `@ember/debug`.
    setTesting(value);
  }
}
