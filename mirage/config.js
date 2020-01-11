import * as Categories from './route-handlers/categories';
import * as Crates from './route-handlers/crates';
import * as Keywords from './route-handlers/keywords';
import * as Summary from './route-handlers/summary';
import * as Teams from './route-handlers/teams';
import * as Users from './route-handlers/users';

export default function() {
  Categories.register(this);
  Crates.register(this);
  Keywords.register(this);
  Summary.register(this);
  Teams.register(this);
  Users.register(this);

  // Used by ember-cli-code-coverage
  this.passthrough('/write-coverage');
}
