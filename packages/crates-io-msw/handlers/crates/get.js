import { http, HttpResponse } from 'msw';

import { db } from '../../index.js';
import { serializeCategory } from '../../serializers/category.js';
import { serializeCrate } from '../../serializers/crate.js';
import { serializeKeyword } from '../../serializers/keyword.js';
import { serializeVersion } from '../../serializers/version.js';
import { notFound } from '../../utils/handlers.js';

const DEFAULT_INCLUDES = ['versions', 'keywords', 'categories'];

export default http.get('/api/v1/crates/:name', async ({ request, params }) => {
  let { name } = params;
  let canonicalName = toCanonicalName(name);
  let crate = db.crate.findMany({}).find(it => toCanonicalName(it.name) === canonicalName);
  if (!crate) return notFound();

  let versions = db.version.findMany({ where: { crate: { id: { equals: crate.id } } } });
  versions.sort((a, b) => b.id - a.id);

  let url = new URL(request.url);
  let include = url.searchParams.get('include');
  let includes = include == null || include === 'full' ? DEFAULT_INCLUDES : include.split(',');

  let includeCategories = includes.includes('categories');
  let includeKeywords = includes.includes('keywords');
  let includeVersions = includes.includes('versions');
  let includeDefaultVersion = includes.includes('default_version');

  let serializedCrate = serializeCrate(crate, {
    calculateVersions: includeVersions,
    includeCategories,
    includeKeywords,
    includeVersions,
  });

  let serializedVersions = null;
  if (includeVersions) {
    serializedVersions = versions.map(v => serializeVersion(v));
  } else if (includeDefaultVersion) {
    let defaultVersion = versions.find(v => v.num === serializedCrate.default_version);
    serializedVersions = [serializeVersion(defaultVersion)];
  }

  return HttpResponse.json({
    crate: serializedCrate,
    categories: includeCategories ? crate.categories.map(c => serializeCategory(c)) : null,
    keywords: includeKeywords ? crate.keywords.map(k => serializeKeyword(k)) : null,
    versions: serializedVersions,
  });
});

function toCanonicalName(name) {
  return name.toLowerCase().replace(/-/g, '_');
}
