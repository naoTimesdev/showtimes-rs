# showtimes-rs

A big rewrite in progress...

## Requirements

The following project was built with the following requirements in mind:
- Rust `1.88.0` or newer
- MongoDB 6.x or newer
- Redis/Valkey 7.x or newer (Redis (Labs) 7.4+ CE is NOT supported/recommended)
- Meilisearch 1.8.x or newer
- ClickHouse 24.x
- S3-compatible storage (e.g. MinIO, AWS S3, Wasabi, etc.) [`Optional`]

### MSRV

The minimum supported Rust version is `1.88.0`

## Usages
1. Clone the repository
2. Do configuration using `config.toml` file (see `config.toml.example` for reference)
3. Build the binary with `cargo build --locked --release --bin showtimes`
   - Or use the `--profile production` flag for a more optimized build
4. Run the binary with: `./target/release/showtimes`
   - Or in Windows: `.\target\release\showtimes.exe`
   - For production build, use `./target/production/showtimes` (or `.\target\production\showtimes.exe`)
5. Access the API at `http://127.0.0.1:5560` by default

## License

This project has been dual-licensed under the [MPL 2.0](https://github.com/naoTimesdev/showtimes-rs/blob/master/LICENSE-MPL) and [AGPL 3.0 only](https://github.com/naoTimesdev/showtimes-rs/blob/master/LICENSE-AGPL) license. Anyone is free to use and redistribute this project and make sure to link back to the original project. More info: [Mozilla Public License 2.0](https://www.tldrlegal.com/license/mozilla-public-license-2-0-mpl-2) and [GNU Affero General Public License v3](https://www.tldrlegal.com/license/gnu-affero-general-public-license-v3-agpl-3-0)

Please refer to each crates for their respective licenses used since this project is a monorepo.
