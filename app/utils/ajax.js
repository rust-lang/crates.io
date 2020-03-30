import fetch from 'fetch';

export default async function ajax(input, init) {
  let response = await fetch(input, init);
  if (response.ok) {
    return await response.json();
  }
  throw response;
}
