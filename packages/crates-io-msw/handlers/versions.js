import dependencies from './versions/dependencies.js';
import downloads from './versions/downloads.js';
import getVersion from './versions/get.js';
import listVersions from './versions/list.js';
import readme from './versions/readme.js';
import unyankVersion from './versions/unyank.js';
import yankVersion from './versions/yank.js';

export default [listVersions, getVersion, yankVersion, unyankVersion, dependencies, downloads, readme];
