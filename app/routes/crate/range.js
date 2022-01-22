import Route from '@ember/routing/route';
import { inject as service } from '@ember/service';

import maxSatisfying from 'semver/ranges/max-satisfying';

function cargoRangeToNpm(range) {
  return range.replace(',', ' ');
}

export default class VersionRoute extends Route {
  @service notifications;
  @service router;

  async model({ range }, transition) {
    let crate = this.modelFor('crate');

    let versions = await crate.get('versions');
    let allVersionNums = versions.map(it => it.num);
    let unyankedVersionNums = versions.filter(it => !it.yanked).map(it => it.num);

    let npmRange = cargoRangeToNpm(range);
    // find a version that matches the specified range
    let versionNum = maxSatisfying(unyankedVersionNums, npmRange) ?? maxSatisfying(allVersionNums, npmRange);
    if (versionNum) {
      this.router.replaceWith('crate.version', versionNum);
    } else {
      let title = `${crate.name}: No matching version found for ${range}`;
      this.router.replaceWith('catch-all', { transition, title });
    }
  }
}
