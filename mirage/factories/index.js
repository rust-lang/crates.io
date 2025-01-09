import apiToken from './api-token';
import category from './category';
import crate from './crate';
import crateOwnerInvitation from './crate-owner-invitation';
import crateOwnership from './crate-ownership';
import dependency from './dependency';
import keyword from './keyword';
import mirageSession from './mirage-session';
import team from './team';
import user from './user';
import version from './version';
import versionDownload from './version-download';

const FACTORIES = {
  apiToken,
  category,
  crate,
  crateOwnerInvitation,
  crateOwnership,
  dependency,
  keyword,
  mirageSession,
  team,
  user,
  version,
  versionDownload,
};

export default FACTORIES;
