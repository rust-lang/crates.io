import apiToken from './api-token';
import category from './category';
import crate from './crate';
import crateOwnerInvitation from './crate-owner-invitation';
import dependency from './dependency';
import keyword from './keyword';
import team from './team';
import user from './user';
import version from './version';
import versionDownload from './version-download';

const SERIALIZERS = {
  apiToken,
  category,
  crate,
  crateOwnerInvitation,
  dependency,
  keyword,
  team,
  user,
  version,
  versionDownload,
};

export default SERIALIZERS;
