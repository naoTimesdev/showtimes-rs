# showtimes-rs

A big rewrite in progress...

## Requirements

The following project was built with the following requirements in mind:
- Rust `1.80.0` or newer
- MongoDB 6.x or newer
- Redis/Valkey 7.x or newer (Redis 7.4+ CE is NOT supported/recommended)
- Meilisearch 1.8.x or newer
- ClickHouse 24.x
- S3-compatible storage (e.g. MinIO, AWS S3, Wasabi, etc.) [`Optional`]

### MSRV

The minimum supported Rust version is `1.80.0` since we utilize the new [`LazyLock`](https://blog.rust-lang.org/2024/07/25/Rust-1.80.0.html) feature that 
recently was stabilized in Rust `1.80.0`.

## Usages
1. Clone the repository
2. Run `cargo build --release --all`
3. Do configuration using `config.toml` file (see `config.toml.example` for reference)
4. Run the binary with `cargo run --release --bin showtimes`
   - Or run the binary directly: `./target/release/showtimes`
   - Or in Windows: `.\target\release\showtimes.exe`
5. Access the API at `http://127.0.0.1:5560` by default

## License

This project has been dual-licensed under the [MPL 2.0](https://github.com/naoTimesdev/showtimes-rs/blob/master/LICENSE-MPL) and [AGPL 3.0 only](https://github.com/naoTimesdev/showtimes-rs/blob/master/LICENSE-MPL) license. Anyone is free to use and redistribute this project and make sure to link back to the original project. More info: [Mozilla Public License 2.0](https://www.tldrlegal.com/license/mozilla-public-license-2-0-mpl-2) and [GNU Affero General Public License v3](https://www.tldrlegal.com/license/gnu-affero-general-public-license-v3-agpl-3-0)

Please refer to each crates for their respective licenses used since this project is a monorepo.
