export async function init() {
  // Clear the bootstrap error handlers set in `app.html`; the app has loaded
  // successfully, so the load-failure fallback is no longer needed.
  // eslint-disable-next-line unicorn/prefer-add-event-listener
  globalThis.onerror = null;
  globalThis.onunhandledrejection = null;

  if (import.meta.env.VITE_MSW_ENABLED) {
    let { http, passthrough } = await import('msw');
    let { setupWorker } = await import('msw/browser');
    let { HttpNetworkFrame } = await import('msw/experimental');
    let { handlers, db } = await import('@crates-io/msw');
    let { loadFixtures } = await import('@crates-io/msw/fixtures');

    let worker = setupWorker(
      ...handlers,
      http.get('https://:avatars.githubusercontent.com/u/:id', passthrough),
      http.get('https://code.cdn.mozilla.net/fonts/*', passthrough),
    );

    await worker.start({
      onUnhandledFrame({ frame, defaults }) {
        if (frame instanceof HttpNetworkFrame && !frame.data.request.url.startsWith(globalThis.location.origin)) {
          defaults.error();
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
