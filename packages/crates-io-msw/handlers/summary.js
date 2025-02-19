import { http, HttpResponse } from 'msw';

import { db } from '../index.js';
import { serializeCategory } from '../serializers/category.js';
import { serializeCrate } from '../serializers/crate.js';
import { serializeKeyword } from '../serializers/keyword.js';
import { compareDates } from '../utils/dates.js';

export default [
  http.get('/api/v1/summary', () => {
    let crates = db.crate.findMany({});

    let just_updated = crates.sort((a, b) => compareDates(b.updated_at, a.updated_at)).slice(0, 10);
    let most_downloaded = crates.sort((a, b) => b.downloads - a.downloads).slice(0, 10);
    let new_crates = crates.sort((a, b) => b.id - a.id).slice(0, 10);
    let most_recently_downloaded = crates.sort((a, b) => b.recent_downloads - a.recent_downloads).slice(0, 10);

    let num_crates = crates.length;
    let num_downloads = crates.reduce((sum, crate) => sum + crate.downloads, 0);

    let popularCategories = db.category.findMany({ take: 10 });
    let popularKeywords = db.keyword.findMany({ take: 10 });

    return HttpResponse.json({
      just_updated: just_updated.map(c => serializeCrate(c)),
      most_downloaded: most_downloaded.map(c => serializeCrate(c)),
      new_crates: new_crates.map(c => serializeCrate(c)),
      most_recently_downloaded: most_recently_downloaded.map(c => serializeCrate(c)),
      num_crates,
      num_downloads,
      popular_categories: popularCategories.map(it => serializeCategory(it)),
      popular_keywords: popularKeywords.map(it => serializeKeyword(it)),
    });
  }),
];
