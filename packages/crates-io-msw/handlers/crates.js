import deleteCrate from './crates/delete.js';
import followCrate from './crates/follow.js';
import following from './crates/following.js';
import getCrate from './crates/get.js';
import listCrates from './crates/list.js';
import unfollowCrate from './crates/unfollow.js';
import userOwners from './crates/user-owners.js';

export default [listCrates, getCrate, deleteCrate, following, followCrate, unfollowCrate, userOwners];
