import { createServer } from 'miragejs';

import * as Categories from './route-handlers/categories';
import * as Crates from './route-handlers/crates';
import * as DocsRS from './route-handlers/docs-rs';
import * as Invites from './route-handlers/invites';
import * as Keywords from './route-handlers/keywords';
import * as Me from './route-handlers/me';
import * as Metadata from './route-handlers/metadata';
import * as Session from './route-handlers/session';
import * as Summary from './route-handlers/summary';
import * as Teams from './route-handlers/teams';
import * as Users from './route-handlers/users';

export default function makeServer(config) {
  return createServer({
    ...config,

    routes() {
      Categories.register(this);
      Crates.register(this);
      DocsRS.register(this);
      Invites.register(this);
      Keywords.register(this);
      Metadata.register(this);
      Me.register(this);
      Session.register(this);
      Summary.register(this);
      Teams.register(this);
      Users.register(this);

      // Used by ember-cli-code-coverage
      this.passthrough('/write-coverage');
    },
  });
}
