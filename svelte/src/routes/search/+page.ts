export function load({ url }) {
  let q = url.searchParams.get('q') ?? '';
  let page = parseInt(url.searchParams.get('page') ?? '1', 10);
  let perPage = 10;
  let sort = url.searchParams.get('sort') ?? 'relevance';
  let allKeywords = url.searchParams.get('all_keywords');

  return { q, page, perPage, sort, allKeywords };
}
