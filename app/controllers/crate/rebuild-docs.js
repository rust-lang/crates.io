import Controller from '@ember/controller';
import { service } from '@ember/service';

import { keepLatestTask } from 'ember-concurrency';

export default class RebuildDocsController extends Controller {
  @service notifications;
  @service router;

  rebuildTask = keepLatestTask(async () => {
    let { version } = this.model;
    try {
      await version.rebuildDocs();
      this.notifications.success('Docs rebuild task was enqueued successfully!');
      this.router.transitionTo('crate.versions', version.crate.name);
    } catch (error) {
      let reason = error?.errors?.[0]?.detail ?? 'Failed to enqueue docs rebuild task.';
      let msg = `Error: ${reason}`;
      this.notifications.error(msg);
    }
  });
}
