const BANNER_MESSAGE_COOKIE_NAME = 'dismissed-banner-messages';

export interface CookieStore {
  get(name: string): Promise<Item | null>;
  set(name: string, value: string): Promise<void>;
}

export interface Item {
  value?: string;
}

async function hash(content: string) {
  let input = new TextEncoder().encode(content);
  let output = await window.crypto.subtle.digest('SHA-256', input);
  return new Uint8Array(output).toHex();
}

async function read(cookieStore: CookieStore) {
  let raw = await cookieStore.get(BANNER_MESSAGE_COOKIE_NAME);
  return new Set(raw?.value?.split(','));
}

async function write(cookieStore: CookieStore, set: Set<string>) {
  await cookieStore.set(BANNER_MESSAGE_COOKIE_NAME, [...set.values()].join(','));
}

async function has(cookieStore: CookieStore, content: string) {
  let id = await hash(content);
  let set = await read(cookieStore);

  return set.has(id);
}

async function set(cookieStore: CookieStore, content: string) {
  let id = await hash(content);
  let set = await read(cookieStore);

  set.add(id);
  await write(cookieStore, set);
}

export default { has, set };
