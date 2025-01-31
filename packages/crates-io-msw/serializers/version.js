import { serializeModel } from '../utils/serializers.js';
import { serializeUser } from './user.js';

export function serializeVersion(version) {
  let serialized = serializeModel(version);

  serialized.crate = version.crate.name;
  serialized.dl_path = `/api/v1/crates/${version.crate.name}/${version.num}/download`;
  serialized.readme_path = `/api/v1/crates/${version.crate.name}/${version.num}/readme`;

  serialized.links = {
    dependencies: `/api/v1/crates/${version.crate.name}/${version.num}/dependencies`,
    version_downloads: `/api/v1/crates/${version.crate.name}/${version.num}/downloads`,
  };

  serialized.published_by = version.publishedBy ? serializeUser(version.publishedBy) : null;

  delete serialized.readme;

  return serialized;
}
