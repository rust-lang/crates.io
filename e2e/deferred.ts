export function defer<T = undefined>(): { resolve: (any?) => T; reject: (reason: any) => void; promise: Promise<T> } {
  let resolve, reject;
  let promise = new Promise((res, rej) => {
    resolve = res;
    reject = rej;
  }) as Promise<T>;
  return { resolve, reject, promise };
}
