import { notFound, pageParams } from './-utils';

export function register(server) {
  server.get('/api/v1/keywords', function (schema, request) {
    let { start, end } = pageParams(request);

    let allKeywords = schema.keywords.all().sort((a, b) => a.crates_cnt - b.crates_cnt);
    let keywords = allKeywords.slice(start, end);
    let total = allKeywords.length;

    return { ...this.serialize(keywords), meta: { total } };
  });

  server.get('/api/v1/keywords/:keyword_id', (schema, request) => {
    let keywordId = request.params.keyword_id;
    let keyword = schema.keywords.find(keywordId);
    return keyword ? keyword : notFound();
  });
}
