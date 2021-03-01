import { runInDebug } from '@ember/debug';

import fetch from 'fetch';

export default async function ajax(input, init) {
  let method = init?.method ?? 'GET';

  let cause;
  try {
    let response = await fetch(input, init);
    if (response.ok) {
      return await response.json();
    }
    cause = new HttpError({ url: input, method, response });
  } catch (error) {
    cause = error;
  }

  throw new AjaxError({ url: input, method, cause });
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

export class AjaxError extends Error {
  constructor({ url, method, cause }) {
    let message = `${method} ${url} failed`;
    runInDebug(() => {
      if (cause?.stack) {
        message += `\n\ncaused by: ${cause.stack}`;
      }
    });

    super(message);
    this.name = 'AjaxError';
    this.method = method;
    this.url = url;
    this.cause = cause;
  }

  get isJsonError() {
    return this.cause instanceof SyntaxError;
  }

  get isNetworkError() {
    return this.cause instanceof TypeError;
  }

  get isHttpError() {
    return this.cause instanceof HttpError;
  }

  get isServerError() {
    return this.isHttpError && this.cause.response.status >= 500 && this.cause.response.status < 600;
  }

  get isClientError() {
    return this.isHttpError && this.cause.response.status >= 400 && this.cause.response.status < 500;
  }

  async json() {
    try {
      return await this.cause.response.json();
    } catch {
      // ignore errors and implicitly return `undefined`
    }
  }
}
