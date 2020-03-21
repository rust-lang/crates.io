import Component from '@ember/component';
import { task } from 'ember-concurrency';
import ajax from 'ember-fetch/ajax';

export default class extends Component {
  tagName = '';

  following = false;

  init() {
    super.init(...arguments);
    this.followStateTask.perform();
  }

  @(task(function*() {
    let d = yield ajax(`/api/v1/crates/${this.crate.name}/following`);
    this.set('following', d.following);
  }).drop())
  followStateTask;

  @task(function*() {
    let crate = this.crate;
    if (this.toggleProperty('following')) {
      yield crate.follow();
    } else {
      yield crate.unfollow();
    }
  })
  toggleFollowTask;
}
