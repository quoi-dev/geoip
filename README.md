# GeoIP service

## Configuration

- `DATA_DIR` (required) - directory with `.mmdb` files,
  must be writable if auto-updates are enabled
- `MAXMIND_ACCOUNT_ID` (optional) - MaxMind account id
- `MAXMIND_LICENCE_KEY` (optional) - MaxMind license key
- `MAXMIND_EDITIONS` (optional) - Comma-separated MaxMind 
  database editions to use (defaults to `GeoLite2-City`)
- `MAXMIND_DOWNLOAD_URL` (optional) - MaxMind database
  download url. You can use `{edition}` placeholder.
  Defaults to `https://download.maxmind.com/geoip/databases/{edition}/download?suffix=tar.gz`

If `MAXMIND_ACCOUNT_ID` or `MAXMIND_DOWNLOAD_URL` are set, 
automatic updates are enabled.

## Tech stack

- Rust
- Axum
