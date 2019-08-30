function decodeFragmentValue(hash) {
  try {
    return decodeURIComponent(hash.slice(1));
  } catch (_) {
    return '';
  }
}

function findElementByFragmentName(document, name) {
  if (name === '') {
    return;
  }

  return document.getElementById(name) || document.getElementsByName(name)[0];
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
  if (typeof window === 'undefined' || typeof window.addEventListener === 'undefined') {
    // Don't run this initializer under FastBoot
    return;
  }
  window.addEventListener('hashchange', hashchange);

  // If clicking on a link to the same fragment as currently in the address bar,
  // hashchange won't be fired, so we need to manually trigger rescroll.
  document.addEventListener('click', function(event) {
    if (event.target.tagName !== 'A') {
      return;
    }
    if (this.href === location.href && location.hash.length > 1) {
      setTimeout(function() {
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
