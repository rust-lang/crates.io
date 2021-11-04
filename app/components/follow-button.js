import { inject as service } from '@ember/service';
import Component from '@glimmer/component';
import { tracked } from '@glimmer/tracking';

import { didCancel, dropTask, task } from 'ember-concurrency';

import ajax from '../utils/ajax';

export default class extends Component {
  @service notifications;

  @tracked following = false;

  constructor() {
    super(...arguments);

    this.followStateTask.perform().catch(error => {
      if (!didCancel(error)) {
        this.notifications.error(
          `Something went wrong while trying to figure out if you are already following the ${this.args.crate.name} crate. Please try again later!`,
        );
      }
    });
  }

  @dropTask *followStateTask() {
    let d = yield ajax(`/api/v1/crates/${this.args.crate.name}/following`);
    this.following = d.following;
  }

  @task *toggleFollowTask() {
    let crate = this.args.crate;

    try {
      yield this.following ? crate.unfollow() : crate.follow();
      this.following = !this.following;
    } catch {
      this.notifications.error(
        `Something went wrong when ${this.following ? 'unfollowing' : 'following'} the ${
          crate.name
        } crate. Please try again later!`,
      );
    }
  }
}
