import CATEGORIES from './fixtures/categories.js';
import CRATE_OWNERSHIPS from './fixtures/crate-ownerships.js';
import CRATES from './fixtures/crates.js';
import DEPENDENCIES from './fixtures/dependencies.js';
import KEYWORDS from './fixtures/keywords.js';
import TEAMS from './fixtures/teams.js';
import USERS from './fixtures/users.js';
import VERSION_DOWNLOADS from './fixtures/version-downloads.js';
import VERSIONS from './fixtures/versions.js';

export function loadFixtures(db) {
  CATEGORIES.forEach(it => db.category.create(it));
  let keywords = KEYWORDS.map(it => db.keyword.create(it));

  let users = USERS.map(it => db.user.create(it));
  let teams = TEAMS.map(it => db.team.create(it));

  let crates = CRATES.map(it => {
    if (it.keywordIds) {
      it.keywords = it.keywordIds.map(id => keywords.find(k => k.id === id)).filter(Boolean);
      delete it.keywordIds;
    }

    return db.crate.create(it);
  });

  CRATE_OWNERSHIPS.forEach(it => {
    if (it.crateId) {
      it.crate = crates.find(c => c.name === it.crateId);
      delete it.crateId;
    }
    if (it.teamId) {
      it.team = teams.find(t => t.id === it.teamId);
      delete it.teamId;
    }
    if (it.userId) {
      it.user = users.find(u => u.id === it.userId);
      delete it.userId;
    }

    return db.crateOwnership.create(it);
  });

  let versions = VERSIONS.map(it => {
    if (it.crateId) {
      it.crate = crates.find(c => c.name === it.crateId);
      delete it.crateId;
    }

    return db.version.create(it);
  });

  DEPENDENCIES.forEach(it => {
    if (it.crateId) {
      it.crate = crates.find(c => c.name === it.crateId);
      delete it.crateId;
    }
    if (it.versionId) {
      it.version = versions.find(v => v.id === it.versionId);
      delete it.versionId;
    }

    return db.dependency.create(it);
  });

  VERSION_DOWNLOADS.forEach(it => {
    if (it.versionId) {
      it.version = versions.find(v => v.id === it.versionId);
      delete it.versionId;
    }

    return db.versionDownload.create(it);
  });
}
