// see https://choosealicense.com/appendix/
const CAL_LICENSES = [
  '0bsd',
  'afl-3.0',
  'agpl-3.0',
  'apache-2.0',
  'artistic-2.0',
  'bsd-2-clause',
  'bsd-3-clause-clear',
  'bsd-3-clause',
  'bsd-4-clause',
  'bsl-1.0',
  'cc-by-4.0',
  'cc-by-sa-4.0',
  'cc0-1.0',
  'cecill-2.1',
  'ecl-2.0',
  'epl-1.0',
  'epl-2.0',
  'eupl-1.1',
  'eupl-1.2',
  'gpl-2.0',
  'gpl-3.0',
  'isc',
  'lgpl-2.1',
  'lgpl-3.0',
  'lppl-1.3c',
  'mit',
  'mpl-2.0',
  'ms-pl',
  'ms-rl',
  'ncsa',
  'odbl-1.0',
  'ofl-1.1',
  'osl-3.0',
  'postgresql',
  'unlicense',
  'upl-1.0',
  'vim',
  'wtfpl',
  'zlib',
];

const LICENSE_KEYWORDS = new Set(['OR', 'AND', 'WITH', '(', ')']);

export function parseLicense(text) {
  return text
    .trim()
    .replace('/', ' OR ')
    .replace(/(^\(| \()/, ' ( ')
    .replace(/(\)$|\) )/, ' ) ')
    .replace(/ +/g, ' ')
    .split(' ')
    .filter(Boolean)
    .map(text => {
      let lowerCaseText = text.toLowerCase();

      let isKeyword = LICENSE_KEYWORDS.has(text);
      let calLicense = CAL_LICENSES.find(it => it === lowerCaseText);
      let link = calLicense ? `https://choosealicense.com/licenses/${calLicense}` : undefined;

      return { isKeyword, text, link };
    });
}
