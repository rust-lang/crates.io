import Component from '@glimmer/component';
import { tracked } from '@glimmer/tracking';
import { task } from 'ember-concurrency';
import ajax from 'ember-fetch/ajax';

export default class extends Component {
  @tracked following = false;

  constructor() {
    super(...arguments);
    this.followStateTask.perform();
  }

  @(task(function* () {
    let d = yield ajax(`/api/v1/crates/${this.args.crate.name}/following`);
    this.following = d.following;
  }).drop())
  followStateTask;

  @task(function* () {
    let crate = this.args.crate;

    if (!this.following) {
      yield crate.follow();
    } else {
      yield crate.unfollow();
    }

    this.following = !this.following;
  })
  toggleFollowTask;
}
