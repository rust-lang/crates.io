import fetch from 'fetch';

export default async function ajax(input, init) {
  let response = await fetch(input, init);
  if (response.ok) {
    return await response.json();
  }
  throw new HttpError(input, init, response);
}

export class HttpError extends Error {
  constructor(url, init, response) {
    let method = init.method ?? 'GET';
    let message = `${method} ${url} failed with: ${response.status} ${response.statusText}`;
    super(message);
    this.name = 'HttpError';
    this.method = method;
    this.url = url;
    this.response = response;
  }
}
