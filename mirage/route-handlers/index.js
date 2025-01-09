import * as Categories from './categories';
import * as Crates from './crates';
import * as DocsRS from './docs-rs';
import * as Invites from './invites';
import * as Keywords from './keywords';
import * as Me from './me';
import * as Metadata from './metadata';
import * as Session from './session';
import * as Summary from './summary';
import * as Teams from './teams';
import * as Users from './users';

export function register(server) {
  Categories.register(server);
  Crates.register(server);
  DocsRS.register(server);
  Invites.register(server);
  Keywords.register(server);
  Metadata.register(server);
  Me.register(server);
  Session.register(server);
  Summary.register(server);
  Teams.register(server);
  Users.register(server);
}
