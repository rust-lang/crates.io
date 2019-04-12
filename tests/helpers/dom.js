// Because Testem adds own title element
// https://github.com/adopted-ember-addons/ember-page-title/blob/4732cdb2c9f673e4714334b33c5b4c5056dfcb8f/tests/acceptance/posts-test.js#L11
function title() {
  let titles = document.head.getElementsByTagName('title');
  return titles[titles.length - 1].innerText;
}

export { title };
