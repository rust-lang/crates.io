import fetch from 'fetch';

export default async function ajax(input, init) {
  let response = await fetch(input, init);
  if (response.ok) {
    return await response.json();
  }
  throw new HttpError({ url: input, method: init?.method ?? 'GET', response });
}

export class HttpError extends Error {
  constructor({ url, method, response }) {
    let message = `${method} ${url} failed with: ${response.status} ${response.statusText}`;
    super(message);
    this.name = 'HttpError';
    this.method = method;
    this.url = url;
    this.response = response;
  }
}
