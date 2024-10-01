import { getOwner } from '@ember/owner';
import Service from '@ember/service';

export default class PristineParamsService extends Service {
  paramsFor(route) {
    let params = getOwner(this).lookup(`controller:${route}`)?.queryParams || [];
    return Object.fromEntries(params.map(k => [k, null]));
  }
}
