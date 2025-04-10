import { service } from '@ember/service';
import { waitForPromise } from '@ember/test-waiters';

import Modifier from 'ember-modifier';

export default class ScrollPositionModifier extends Modifier {
  @service notifications;
  @service mermaid;

  modify(element) {
    // If the `mermaid` library is loaded (which should have happened in the controller)
    let mermaid = this.mermaid.loadTask.lastSuccessful?.value;
    if (mermaid) {
      // ... find any relevant code snippets
      let nodes = element.querySelectorAll('.language-mermaid');

      // ... and render them as diagrams
      waitForPromise(mermaid.run({ nodes })).catch(error => {
        // Log errors to the console
        console.error(error.error || error);

        // ... and report them as warnings to the user
        this.notifications.warning('Failed to render mermaid diagram.');
      });
    }
  }
}
