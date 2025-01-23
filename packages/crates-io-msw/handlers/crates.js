import addOwners from './crates/add-owners.js';
import deleteCrate from './crates/delete.js';
import downloads from './crates/downloads.js';
import followCrate from './crates/follow.js';
import following from './crates/following.js';
import getCrate from './crates/get.js';
import listCrates from './crates/list.js';
import removeOwners from './crates/remove-owners.js';
import reverseDependencies from './crates/reverse-dependencies.js';
import teamOwners from './crates/team-owners.js';
import unfollowCrate from './crates/unfollow.js';
import userOwners from './crates/user-owners.js';

export default [
  listCrates,
  getCrate,
  deleteCrate,
  following,
  followCrate,
  unfollowCrate,
  addOwners,
  removeOwners,
  userOwners,
  teamOwners,
  reverseDependencies,
  downloads,
];
