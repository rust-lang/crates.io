import macro from 'macro-decorators';

const VIEWABLE_PAGES = 9;

export function pagination() {
  return macro(function () {
    let { page, per_page: perPage, totalItems } = this;
    return _pagination(page, perPage, totalItems);
  });
}

function _pagination(page, perPage, totalItems) {
  let currentPage = parseInt(page, 10) || 1;

  let currentPageStart = totalItems === 0 ? 0 : (currentPage - 1) * perPage + 1;
  let currentPageEnd = Math.min(currentPage * perPage, totalItems);

  let availablePages = Math.ceil(totalItems / perPage || 1);

  let nextPage = currentPage + 1;
  if (nextPage > availablePages) {
    nextPage = currentPage;
  }

  let prevPage = currentPage - 1;
  if (prevPage <= 0) {
    prevPage = currentPage;
  }

  // Always show the same number of pages even if we're
  // at the beginning or at the end of the list.
  let lowerBound, upperBound;
  if (availablePages - currentPage < Math.ceil(VIEWABLE_PAGES / 2)) {
    lowerBound = Math.max(0, availablePages - VIEWABLE_PAGES);
    upperBound = availablePages;
  } else if (currentPage <= Math.ceil(VIEWABLE_PAGES / 2)) {
    lowerBound = 0;
    upperBound = Math.min(availablePages, VIEWABLE_PAGES);
  } else {
    lowerBound = currentPage - Math.ceil(VIEWABLE_PAGES / 2);
    upperBound = currentPage + Math.floor(VIEWABLE_PAGES / 2);
  }

  let pages = [];
  for (let i = lowerBound; i < upperBound; i++) {
    pages.push(i + 1);
  }

  return {
    currentPage,
    currentPageStart,
    currentPageEnd,
    availablePages,
    nextPage,
    prevPage,
    pages,
  };
}
