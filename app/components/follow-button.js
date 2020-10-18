import Component from '@glimmer/component';
import { tracked } from '@glimmer/tracking';

import { task } from 'ember-concurrency';

import ajax from '../utils/ajax';

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

    yield !this.following ? crate.follow() : crate.unfollow();

    this.following = !this.following;
  })
  toggleFollowTask;
}
