import window from 'ember-window-mock';

export function getItem(key) {
  try {
    return window.localStorage.getItem(key);
  } catch (e) {
    return null;
  }
}

export function setItem(key, value) {
  try {
    window.localStorage.setItem(key, value);
  } catch (e) {
    // ignored
  }
}

export function removeItem(key) {
  try {
    window.localStorage.removeItem(key);
  } catch (e) {
    // ignored
  }
}
