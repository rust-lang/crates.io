import macro from 'macro-decorators';

export function pagination() {
  return macro(function () {
    let { nextPage, totalItems } = this;

    return {
      nextPage,
      totalItems,
    };
  });
}
