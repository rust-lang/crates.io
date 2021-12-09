import Route from '@ember/routing/route';
import { inject as service } from '@ember/service';

import maxSatisfying from 'semver/ranges/max-satisfying';

function cargoRangeToNpm(range) {
  return range.replace(',', ' ');
}

export default class VersionRoute extends Route {
  @service notifications;
  @service router;

  async model({ range }) {
    let crate = this.modelFor('crate');

    let versions = await crate.get('versions');
    let allVersionNums = versions.map(it => it.num);
    let unyankedVersionNums = versions.filter(it => !it.yanked).map(it => it.num);

    let npmRange = cargoRangeToNpm(range);
    // find a version that matches the specified range
    let versionNum = maxSatisfying(unyankedVersionNums, npmRange) ?? maxSatisfying(allVersionNums, npmRange);
    if (!versionNum) {
      this.notifications.error(`No matching version of crate '${crate.name}' found for: ${range}`);
      this.router.replaceWith('crate.index');
    }

    this.router.replaceWith('crate.version', versionNum);
  }
}
