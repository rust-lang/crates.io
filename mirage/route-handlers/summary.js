import { compareIsoDates } from './-utils';

export function register(server) {
  server.get('/api/v1/summary', function(schema) {
    let crates = schema.crates.all();

    let just_updated = crates.sort((a, b) => compareIsoDates(b.updated_at, a.updated_at)).slice(0, 10);
    let most_downloaded = crates.sort((a, b) => b.downloads - a.downloads).slice(0, 10);
    let new_crates = crates.sort((a, b) => compareIsoDates(b.created_at, a.created_at)).slice(0, 10);
    let most_recently_downloaded = crates.sort((a, b) => b.recent_downloads - a.recent_downloads).slice(0, 10);

    let num_crates = crates.length;
    let num_downloads = crates.models.reduce((sum, crate) => sum + crate.downloads, 0);

    let popular_categories = schema.categories
      .all()
      .sort((a, b) => b.crates_cnt - a.crates_cnt)
      .slice(0, 10);
    let popular_keywords = schema.keywords
      .all()
      .sort((a, b) => b.crates_cnt - a.crates_cnt)
      .slice(0, 10);

    return {
      just_updated: this.serialize(just_updated).crates.map(it => ({ ...it, versions: null })),
      most_downloaded: this.serialize(most_downloaded).crates.map(it => ({ ...it, versions: null })),
      new_crates: this.serialize(new_crates).crates.map(it => ({ ...it, versions: null })),
      most_recently_downloaded: this.serialize(most_recently_downloaded).crates.map(it => ({
        ...it,
        versions: null,
      })),
      num_crates,
      num_downloads,
      popular_categories: this.serialize(popular_categories).categories,
      popular_keywords: this.serialize(popular_keywords).keywords,
    };
  });
}
