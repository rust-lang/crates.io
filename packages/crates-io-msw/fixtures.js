import CATEGORIES from './fixtures/categories.js';
import CRATE_OWNERSHIPS from './fixtures/crate-ownerships.js';
import CRATES from './fixtures/crates.js';
import DEPENDENCIES from './fixtures/dependencies.js';
import KEYWORDS from './fixtures/keywords.js';
import TEAMS from './fixtures/teams.js';
import USERS from './fixtures/users.js';
import VERSION_DOWNLOADS from './fixtures/version-downloads.js';
import VERSIONS from './fixtures/versions.js';

export async function loadFixtures(db) {
  await Promise.all(structuredClone(CATEGORIES).map(it => db.category.create(it)));
  let keywords = await Promise.all(structuredClone(KEYWORDS).map(it => db.keyword.create(it)));

  let users = await Promise.all(structuredClone(USERS).map(it => db.user.create(it)));
  let teams = await Promise.all(structuredClone(TEAMS).map(it => db.team.create(it)));

  let crates = await Promise.all(
    structuredClone(CRATES).map(it => {
      if (it.keywordIds) {
        it.keywords = it.keywordIds.map(id => keywords.find(k => k.id === id)).filter(Boolean);
        delete it.keywordIds;
      }

      return db.crate.create(it);
    }),
  );

  await Promise.all(
    structuredClone(CRATE_OWNERSHIPS).map(it => {
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
    }),
  );

  let versions = await Promise.all(
    structuredClone(VERSIONS).map(it => {
      if (it.crateId) {
        it.crate = crates.find(c => c.name === it.crateId);
        delete it.crateId;
      }

      return db.version.create(it);
    }),
  );

  await Promise.all(
    structuredClone(DEPENDENCIES).map(it => {
      if (it.crateId) {
        it.crate = crates.find(c => c.name === it.crateId);
        delete it.crateId;
      }
      if (it.versionId) {
        it.version = versions.find(v => v.id === it.versionId);
        delete it.versionId;
      }

      return db.dependency.create(it);
    }),
  );

  await Promise.all(
    structuredClone(VERSION_DOWNLOADS).map(it => {
      if (it.versionId) {
        it.version = versions.find(v => v.id === it.versionId);
        delete it.versionId;
      }

      return db.versionDownload.create(it);
    }),
  );
}
