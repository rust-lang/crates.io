import dependencies from './versions/dependencies.js';
import downloads from './versions/downloads.js';
import followUpdates from './versions/follow-updates.js';
import getVersion from './versions/get.js';
import listVersions from './versions/list.js';
import patchVersion from './versions/patch.js';
import readme from './versions/readme.js';
import rebuildDocs from './versions/rebuild-docs.js';
import unyankVersion from './versions/unyank.js';
import yankVersion from './versions/yank.js';

export default [
  listVersions,
  getVersion,
  patchVersion,
  yankVersion,
  unyankVersion,
  dependencies,
  downloads,
  readme,
  rebuildDocs,
  followUpdates,
];
