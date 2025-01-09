import { compareStrings, notFound, pageParams } from './-utils';

export function register(server) {
  server.get('/api/v1/categories', function (schema, request) {
    let { start, end } = pageParams(request);

    let allCategories = schema.categories.all().sort((a, b) => compareStrings(a.category, b.category));
    let categories = allCategories.slice(start, end);
    let total = allCategories.length;

    return { ...this.serialize(categories), meta: { total } };
  });

  server.get('/api/v1/categories/:category_id', function (schema, request) {
    let catId = request.params.category_id;
    let category = schema.categories.find(catId);
    return category ?? notFound();
  });

  server.get('/api/v1/category_slugs', function (schema) {
    let allCategories = schema.categories.all().sort((a, b) => compareStrings(a.category, b.category));
    return {
      category_slugs: this.serialize(allCategories).categories.map(cat => ({
        id: cat.id,
        slug: cat.slug,
        description: cat.description,
      })),
    };
  });
}
