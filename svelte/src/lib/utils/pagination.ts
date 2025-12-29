const VIEWABLE_PAGES = 9;

export interface PaginationState {
  currentPage: number;
  currentPageStart: number;
  currentPageEnd: number;
  availablePages: number;
  nextPage: number;
  prevPage: number;
  pages: number[];
  maxPages?: number;
}

export function calculatePagination(
  currentPage: number,
  perPage: number,
  totalItems: number,
  maxPages?: number,
): PaginationState {
  let currentPageStart = totalItems === 0 ? 0 : (currentPage - 1) * perPage + 1;
  let currentPageEnd = Math.min(currentPage * perPage, totalItems);

  let calculatedPages = Math.ceil(totalItems / perPage || 1);
  let availablePages = maxPages === undefined ? calculatedPages : Math.min(calculatedPages, maxPages);

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
  let lowerBound: number;
  let upperBound: number;
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

  let pages: number[] = [];
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
    maxPages,
  };
}
