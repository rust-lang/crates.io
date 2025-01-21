export function pageParams(request) {
  let url = new URL(request.url);

  let page = parseInt(url.searchParams.get('page') || '1');
  let perPage = parseInt(url.searchParams.get('per_page') || '10');

  let start = (page - 1) * perPage;
  let end = start + perPage;

  let skip = start;
  let take = perPage;

  return { page, perPage, start, end, skip, take };
}
