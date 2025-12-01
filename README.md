# GeoIP service

Demo: https://geoip.quoi.dev/

## Quick setup

```shell
docker run \
  -e MAXMIND_ACCOUNT_ID=XXXX \
  -e MAXMIND_LICENCE_KEY=YYYY \
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
  (database versions etc)
- `GET /api/ip` - Detect requester IP
- `GET /api/geoip` - Perform GeoIP lookup

Swagger UI is available on `/swagger-ui`,
OpenAPI specification is available on `/api/docs`.

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
  Defaults to `https://download.maxmind.com/geoip/databases/{edition}/download?suffix=tar.gz`
- `AUTO_UPDATE_INTERVAL` (optional) - Auto-update interval 
  in hours. Defaults to 24 hours.
- `API_KEY` (optional) - Protect `/api/geoip` endpoint 
  with given bearer token.

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
