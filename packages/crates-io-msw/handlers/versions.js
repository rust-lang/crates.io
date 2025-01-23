import dependencies from './versions/dependencies.js';
import downloads from './versions/downloads.js';
import getVersion from './versions/get.js';
import listVersions from './versions/list.js';
import yankVersion from './versions/yank.js';

export default [listVersions, getVersion, yankVersion, dependencies, downloads];
