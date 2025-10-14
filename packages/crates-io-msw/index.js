import apiTokenHandlers from './handlers/api-tokens.js';
import categoryHandlers from './handlers/categories.js';
import cratesHandlers from './handlers/crates.js';
import docsRsHandlers from './handlers/docs-rs.js';
import githubHandlers from './handlers/github.js';
import gitlabHandlers from './handlers/gitlab.js';
import inviteHandlers from './handlers/invites.js';
import keywordHandlers from './handlers/keywords.js';
import metadataHandlers from './handlers/metadata.js';
import playgroundHandlers from './handlers/playground.js';
import sessionHandlers from './handlers/sessions.js';
import summaryHandlers from './handlers/summary.js';
import teamHandlers from './handlers/teams.js';
import trustpubHandlers from './handlers/trustpub.js';
import userHandlers from './handlers/users.js';
import versionHandlers from './handlers/versions.js';

export const handlers = [
  ...apiTokenHandlers,
  ...categoryHandlers,
  ...cratesHandlers,
  ...docsRsHandlers,
  ...githubHandlers,
  ...gitlabHandlers,
  ...inviteHandlers,
  ...keywordHandlers,
  ...metadataHandlers,
  ...playgroundHandlers,
  ...sessionHandlers,
  ...summaryHandlers,
  ...teamHandlers,
  ...trustpubHandlers,
  ...userHandlers,
  ...versionHandlers,
];

export { db } from './models/index.js';
