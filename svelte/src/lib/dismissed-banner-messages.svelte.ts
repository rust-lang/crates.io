import { getItem, setItem } from './utils/local-storage';

const BANNER_MESSAGE_STORAGE_KEY = 'dismissed-banner-messages';

async function hash(content: string) {
  let input = new TextEncoder().encode(content);
  let output = await window.crypto.subtle.digest('SHA-256', input);
  return new Uint8Array(output).toHex();
}

function read() {
  let raw = getItem(BANNER_MESSAGE_STORAGE_KEY);
  return new Set(raw?.split(','));
}

function write(set: Set<string>) {
  setItem(BANNER_MESSAGE_STORAGE_KEY, [...set.values()].join(','));
}

async function has(content: string) {
  let id = await hash(content);
  let set = read();

  return set.has(id);
}

async function set(content: string) {
  let id = await hash(content);
  let set = read();

  set.add(id);
  write(set);
}

export default { has, set };
