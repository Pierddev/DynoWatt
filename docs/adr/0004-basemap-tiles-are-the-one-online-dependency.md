# Basemap tiles are the one online dependency

The map an Activity's Track is drawn on needs basemap tiles, and no offline basemap of a sensible size fits in a desktop app. So the map fetches vector tiles from OpenFreeMap and renders them with MapLibre GL JS, and those tile requests are the only network traffic DynoWatt makes. They reveal which area the Athlete is looking at, but nothing else leaves the machine: Source Files, Samples and the Track stay local, and without a network the Track still draws, on a plain background. OpenFreeMap was chosen because it needs no API key, sets no usage limit and has a dark style. A key would ship in the public binary of a GPL app for anyone to reuse.

## Considered Options

- **Google Maps or Mapbox**: both need an API key tied to a billing account, and the key would be exposed in the open-source binary. Google also supports non-HTTP origins such as a Tauri webview poorly.
- **Free tiers that need a key (Stadia Maps, MapTiler, Thunderforest, CARTO)**: the key is still in the binary, and the free tiers are limited to non-commercial or evaluation use, or (Stadia Maps) exclude desktop apps without a paid subscription.
- **Leaflet with OpenStreetMap raster tiles**: light and needs no WebGL, but it has no dark style, OpenStreetMap enforces a strict tile usage policy, and colouring the Track by a channel is clumsier.
- **Offline tiles, bundled or downloaded**: hundreds of megabytes or more, out of proportion for a backdrop.
- **No basemap, the Track alone**: fully local, but loses the roads that give the Track its meaning.

## Consequences

- MapLibre GL 6 requires WebGL2. Where it is missing (some Linux setups with NVIDIA under WebKitGTK), the map column says the map is unavailable, and the charts work as before.
- MapLibre downloads the style as well as the tiles, so the app ships a minimal local style (a plain light or dark background) and switches to it when OpenFreeMap's style cannot be loaded. Offline, the Track, its markers and the hover still work.
- OpenFreeMap is provided as-is and may stop without notice. Its style URL lives in one place, so moving to another provider that serves an OpenMapTiles-compatible style is a one-line change.
- The OpenStreetMap and OpenMapTiles attribution must stay visible on the map.
- The README's "zero cloud lock-in" holds for the Athlete's data, not for the basemap.
