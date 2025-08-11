import { on } from '@ember/modifier';
import { service } from '@ember/service';
import Component from '@glimmer/component';
import { tracked } from '@glimmer/tracking';

import { didCancel, dropTask, task } from 'ember-concurrency';
import perform from 'ember-concurrency/helpers/perform';
import or from 'ember-truth-helpers/helpers/or';

import LoadingSpinner from 'crates-io/components/loading-spinner';

import ajax from '../utils/ajax';

export default class extends Component {
  <template>
    <button
      type='button'
      disabled={{or this.followStateTask.isRunning this.followStateTask.last.error this.toggleFollowTask.isRunning}}
      data-test-follow-button
      ...attributes
      class='follow-button button button--tan'
      {{on 'click' (perform this.toggleFollowTask)}}
    >
      {{#if (or this.followStateTask.isRunning this.toggleFollowTask.isRunning)}}
        <LoadingSpinner @theme='light' data-test-spinner />
      {{else}}
        {{#if this.following}}
          Unfollow
        {{else}}
          Follow
        {{/if}}
      {{/if}}
    </button>
  </template>
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

  followStateTask = dropTask(async () => {
    let d = await ajax(`/api/v1/crates/${this.args.crate.name}/following`);
    this.following = d.following;
  });

  toggleFollowTask = task(async () => {
    let crate = this.args.crate;

    try {
      this.following ? await crate.unfollow() : await crate.follow();
      this.following = !this.following;
    } catch {
      this.notifications.error(
        `Something went wrong when ${this.following ? 'unfollowing' : 'following'} the ${
          crate.name
        } crate. Please try again later!`,
      );
    }
  });
}
