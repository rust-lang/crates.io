export async function init() {
  if (import.meta.env.VITE_MSW_ENABLED) {
    let { http, passthrough } = await import('msw');
    let { setupWorker } = await import('msw/browser');
    let { handlers, db } = await import('@crates-io/msw');
    let { loadFixtures } = await import('@crates-io/msw/fixtures');

    let worker = setupWorker(
      ...handlers,
      http.get('https://:avatars.githubusercontent.com/u/:id', passthrough),
      http.get('https://code.cdn.mozilla.net/fonts/*', passthrough),
    );

    // We need to dynamically import `$app/paths` here to avoid a race condition during app startup,
    // resulting in a "ReferenceError: __SVELTEKIT_PAYLOAD__ is not defined" error.
    let { asset } = await import('$app/paths');

    await worker.start({
      serviceWorker: { url: asset('/mockServiceWorker.js') },
      onUnhandledRequest(request, print) {
        if (!request.url.startsWith(globalThis.location.origin)) {
          print.error();
        }
      },
    });

    await loadFixtures(db);

    // eslint-disable-next-line @typescript-eslint/no-explicit-any
    let user = db.user.findFirst((q: any) => q.where({ id: 2 }));
    if (user) {
      await db.mswSession.create({ user });
      localStorage.setItem('isLoggedIn', '1');
    }
  }
}
