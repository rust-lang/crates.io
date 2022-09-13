import macro from 'macro-decorators';

const VIEWABLE_PAGES = 9;

export function pagination() {
  return macro(function () {
    let { page, per_page: perPage, totalItems } = this;

    return {
        page,
        perPage,
        totalItems,
    }
  });
}
