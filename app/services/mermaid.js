import Service from '@ember/service';

import { dropTask } from 'ember-concurrency';

export default class MermaidService extends Service {
  loadTask = dropTask(async () => {
    let { default: mermaid } = await import('mermaid');
    mermaid.initialize({ startOnLoad: false, securityLevel: 'strict' });
    return mermaid;
  });
}
