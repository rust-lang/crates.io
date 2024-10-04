function decodeFragmentValue(hash) {
  try {
    return decodeURIComponent(hash.slice(1));
  } catch {
    return '';
  }
}

function findElementByFragmentName(document, name) {
  if (name === '') {
    return;
  }

  try {
    // eslint-disable-next-line unicorn/prefer-query-selector
    return document.querySelector(`#${name}`) || document.getElementsByName(name)[0];
  } catch {
    // Catches exceptions thrown when an anchor in a readme was invalid (see issue #3108)
    return;
  }
}

function hashchange() {
  if (document.querySelector(':target')) {
    return;
  }

  const hash = decodeFragmentValue(location.hash);
  const target = findElementByFragmentName(document, `user-content-${hash}`);
  if (target) {
    target.scrollIntoView();
  }
}

export function initialize() {
  window.addEventListener('hashchange', hashchange);

  // If clicking on a link to the same fragment as currently in the address bar,
  // hashchange won't be fired, so we need to manually trigger rescroll.
  document.addEventListener('click', function (event) {
    if (event.target.tagName !== 'A') {
      return;
    }
    if (this.href === location.href && location.hash.length > 1) {
      setTimeout(function () {
        if (!event.defaultPrevented) {
          hashchange();
        }
      });
    }
  });
}

export default {
  name: 'app.hashchange',
  initialize,
};
