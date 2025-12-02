# GeoIP service

Demo: https://geoip.quoi.dev/

## Features

- Very performant thanks to native executable
- Requester IP detection
- GeoIP lookups using MaxMind databases
- Simultaneous serving of several MaxMind database editions
  (City, ASN etc.)
- Automatic updates using MaxMind account and licence key or
  custom download url with or without authorization
- Serve latest MMDB archive files
  - One GeoIP service can supply MMDB files to others in order
    to reduce MaxMind API quota usage
  - GeoIP service can supply up-to-date files to
    third-party applications
- Protect data endpoints with API key
- `posix_timezone` field computed from `timezone`
  (useful for embedded devices without timezone database, 
  e.g. ESP32 and other newlib targets)
- Endpoint exposing list of timezone mappings to
  POSIX spec string
- Fancy Web UI with service status and manual GeoIP lookups
- OpenStreetMap integration for Web UI
- Protect Web UI with Recaptcha v3
- OpenAPI spec and Swagger UI
- Easy-to-use Docker image

## Quick setup

```shell
docker run \
  -e MAXMIND_ACCOUNT_ID=XXXX \
  -e MAXMIND_LICENCE_KEY=YYYY \
  -e OSM_TILES_URL="https://{s}.tile.openstreetmap.org/{z}/{x}/{y}.png" \
  -v geoip_data:/data \
  -p 8080:8080 \
  ghcr.io/quoi-dev/geoip:latest
```

You can get MaxMind account id and licence key for free here:
https://dev.maxmind.com/geoip/geolite2-free-geolocation-data/

After executing this command, you'll be able to access 
GeoIP service on http://localhost:8080/.

## Endpoints

- `GET /api/status` - Query system status 
  (database versions etc).
- `GET /api/ip` - Detect requester IP.
- `GET /api/geoip` - Perform GeoIP lookup.
  Can be protected with API key.
- `GET /files/mmdb/{edition}` - Download latest MMDB database
  compressed into `tar.gz`, supports `If-Modified-Since` header.
  Can be used as `MAXMIND_DOWNLOAD_URL` for other instances.
  Can be protected with API key.
- `GET /api/timezones` - Get all known timezone mappings from
  ids to POSIX specification (useful for embedded systems
  without timezone database).

Swagger UI available on `/swagger-ui`,
OpenAPI specification available on `/api/docs`.

## Configuration

- `LISTEN_ADDR` (optional) - socket address to bind HTTP server.
  Defaults to `127.0.0.1:8080` for local setup and to
  `0.0.0.0:8080` for Docker image.
- `DATA_DIR` (required) - directory with `.mmdb` files,
  must be writable if auto-updates are enabled. Defaults to
  `/data` for Docker image.
- `MAXMIND_ACCOUNT_ID` (optional) - MaxMind account id.
- `MAXMIND_LICENCE_KEY` (optional) - MaxMind license key.
- `MAXMIND_EDITIONS` (optional) - Comma-separated MaxMind 
  database editions to use (defaults to `GeoLite2-City`)
- `MAXMIND_DOWNLOAD_URL` (optional) - MaxMind database
  download url. You can use `{edition}` placeholder.
  Defaults to `https://download.maxmind.com/geoip/databases/{edition}/download?suffix=tar.gz`.
  You can point download url to another GeoIP service instance
  (e.g. `http://my-geoip-svc/files/mmdb/{edition}`).
- `MAXMIND_BEARER_TOKEN` (optional) - Use bearer token for
  `MAXMIND_DOWNLOAD_URL`. Useful if download url points to
  another GeoIP service instance with `API_KEY` set.
- `AUTO_UPDATE_INTERVAL` (optional) - Auto-update interval 
  in hours. Defaults to 24 hours.
- `API_KEY` (optional) - Protect `/api/geoip` and 
  `/files/**` endpoints with given bearer token.
- `RECAPTCHA_SITE_KEY` (optional) - Protect `/api/geoip` endpoint
  with Recaptcha v3. `API_KEY` bypasses captcha check,
  Recaptcha bypasses `API_KEY` requirement (only for `/api/geoip`), 
  if both are set. `RECAPTCHA_SITE_KEY` makes no sense without
  API key set.
  For demo purposes on https://geoip.quoi.dev. You can get
  Recaptcha site key on https://www.google.com/recaptcha/admin/create.
  Recaptcha script will be injected to frontend only 
  if site key is set.
- `RECAPTCHA_SECRET_KEY` (optional) - To be used with `RECAPTCHA_SITE_KEY`.
- `OSM_TILES_URL` (optional) - Render OpenStreetMap centered on detected location.
  You can use `https://{s}.tile.openstreetmap.org/{z}/{x}/{y}.png` as a starting point
  and switch to your own tile server if traffic increases.

If `MAXMIND_ACCOUNT_ID` or `MAXMIND_DOWNLOAD_URL` are set, 
automatic updates are enabled, otherwise you need to download,
extract and place `mmdb` files to `DATA_DIR` by hand.

File names must have format `{edition}-{datetime}.mmdb`.
Example: `GeoLite2-City-20251125154543.mmdb`.

## Build

You need Rust and Node installed on your system.

```shell
npm i -g pnpm # if missing
pnpm install
pnpm openapi-ts # first time or after OpenAPI spec changes
pnpm build
cargo build
```

Alternatively, you can build Docker image 
using provided `Dockerfile`.

## Tech stack

- Rust
- Axum
- Rsbuild
- TypeScript
- React
- TailwindCSS
- DaisyUI
- MaxMind GeoLite2
- OpenAPI
