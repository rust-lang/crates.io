import macro from 'macro-decorators';

export function pagination() {
  return macro(function () {
    const { nextPage } = this;
    const nextPageParams = new URLSearchParams(nextPage);

    return {
      nextPage: nextPageParams.get('seek'),
    };
  });
}
