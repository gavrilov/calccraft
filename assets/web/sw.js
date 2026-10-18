// The web version's service worker: with the app manifest it makes CalcCraft installable, and
// it keeps the files the app needs, so the installed app starts without a connection.
//
// - The page itself comes from the network first, so a new deployment shows on the next
//   start; without a connection, the copy saved last time.
// - Everything else (the .wasm, the .js, the icons) comes from the saved copy first. Trunk names
//   the app's files after their contents (calculator-<hash>_bg.wasm), so a saved file is never
//   out of date, and a new build's files replace the old ones, which are deleted.

const CACHE = "calccraft";

// One copy of the page, whatever the address's parameters (?lang=de-DE).
const PAGE = new URL("./", self.registration.scope).href;

// The app's own files, named after their contents by Trunk.
const BUILD_FILE = /^calculator-[0-9a-f]+(_bg\.wasm|\.js)$/;

self.addEventListener("install", () => self.skipWaiting());

self.addEventListener("activate", (event) => event.waitUntil(self.clients.claim()));

self.addEventListener("fetch", (event) => {
    const request = event.request;
    if (request.method !== "GET" || new URL(request.url).origin !== self.location.origin) {
        return;
    }
    if (request.mode === "navigate") {
        event.respondWith(page(request));
    } else {
        event.respondWith(file(request));
    }
});

async function page(request) {
    const cache = await caches.open(CACHE);
    try {
        const response = await fetch(request);
        if (response.ok) {
            await cache.put(PAGE, response.clone());
            // The build files the new page doesn't use any more.
            const html = await response.clone().text();
            for (const saved of await cache.keys()) {
                const name = new URL(saved.url).pathname.split("/").pop();
                if (BUILD_FILE.test(name) && !html.includes(name)) {
                    await cache.delete(saved);
                }
            }
        }
        return response;
    } catch (offline) {
        const saved = await cache.match(PAGE);
        if (saved) {
            return saved;
        }
        throw offline;
    }
}

async function file(request) {
    const cache = await caches.open(CACHE);
    const saved = await cache.match(request);
    if (saved) {
        return saved;
    }
    const response = await fetch(request);
    if (response.ok) {
        await cache.put(request, response.clone());
    }
    return response;
}
