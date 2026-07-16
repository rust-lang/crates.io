import { Collection } from '@msw/data';

import * as counters from '../utils/counters.js';
import apiTokens from './api-token.js';
import categories from './category.js';
import crateOwnerInvitations from './crate-owner-invitation.js';
import crateOwnerships from './crate-ownership.js';
import crates from './crate.js';
import dependencies from './dependency.js';
import keywords from './keyword.js';
import mswSessions from './msw-session.js';
import teams from './team.js';
import trustpubGithubConfigs from './trustpub/github-config.js';
import trustpubGitlabConfigs from './trustpub/gitlab-config.js';
import users from './user.js';
import versionDownloads from './version-download.js';
import versions from './version.js';

// Define relations
// users.defineRelations(({ many }) => ({
//   followedCrates: many(crates),
// }));

crates.defineRelations(({ many }) => ({
  categories: many(categories),
  keywords: many(keywords),
}));

versions.defineRelations(({ one }) => ({
  crate: one(crates),
  publishedBy: one(users),
}));

dependencies.defineRelations(({ one }) => ({
  crate: one(crates),
  version: one(versions),
}));

versionDownloads.defineRelations(({ one }) => ({
  version: one(versions),
}));

crateOwnerships.defineRelations(({ one }) => ({
  crate: one(crates),
  team: one(teams),
  user: one(users),
}));

apiTokens.defineRelations(({ one }) => ({
  user: one(users),
}));

crateOwnerInvitations.defineRelations(({ one }) => ({
  crate: one(crates),
  invitee: one(users),
  inviter: one(users),
}));

mswSessions.defineRelations(({ one }) => ({
  user: one(users),
}));

trustpubGithubConfigs.defineRelations(({ one }) => ({
  crate: one(crates),
}));

trustpubGitlabConfigs.defineRelations(({ one }) => ({
  crate: one(crates),
}));

export const db = {
  apiToken: apiTokens,
  category: categories,
  crateOwnerInvitation: crateOwnerInvitations,
  crateOwnership: crateOwnerships,
  crate: crates,
  dependency: dependencies,
  keyword: keywords,
  mswSession: mswSessions,
  team: teams,
  trustpubGithubConfig: trustpubGithubConfigs,
  trustpubGitlabConfig: trustpubGitlabConfigs,
  user: users,
  versionDownload: versionDownloads,
  version: versions,

  reset() {
    counters.reset();

    for (let collection of Object.values(this)) {
      if (collection instanceof Collection) {
        collection.clear();
      }
    }
  },
};

// Record types stored in each collection, derived from the valibot schemas.
// These are the values returned by the collection query methods, e.g.
// `db.crate.findFirst()` returns a `Crate`.
export type ApiToken = ReturnType<typeof apiTokens.all>[number];
export type Category = ReturnType<typeof categories.all>[number];
export type Crate = ReturnType<typeof crates.all>[number];
export type CrateOwnerInvitation = ReturnType<typeof crateOwnerInvitations.all>[number];
export type CrateOwnership = ReturnType<typeof crateOwnerships.all>[number];
export type Dependency = ReturnType<typeof dependencies.all>[number];
export type Keyword = ReturnType<typeof keywords.all>[number];
export type MswSession = ReturnType<typeof mswSessions.all>[number];
export type Team = ReturnType<typeof teams.all>[number];
export type TrustpubGithubConfig = ReturnType<typeof trustpubGithubConfigs.all>[number];
export type TrustpubGitlabConfig = ReturnType<typeof trustpubGitlabConfigs.all>[number];
export type User = ReturnType<typeof users.all>[number];
export type Version = ReturnType<typeof versions.all>[number];
export type VersionDownload = ReturnType<typeof versionDownloads.all>[number];
