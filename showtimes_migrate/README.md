# migrate

A migration tooling for Showtimes API.

Recommended to be ran first.

### Configuration

We use `.env` file to run this

```toml
# Your mongodb URL
MONGODB_URI=
# Your old database name that contains showtimes_db collection
OLD_DB_NAME=naotimesdb

# Meilisearch URL
MEILI_URL=
MEILI_KEY=masterkey

# S3/local storages configuration
S3_REGION=eu-central-1
S3_ENDPOINT_URL=
S3_BUCKET=cdn.naoti.me
S3_ACCESS_KEY=
S3_SECRET_KEY=
LOCAL_STORAGE=/path/to/your/local/storage
```

## License

This crates has been licensed under the [AGPL 3.0 only](https://github.com/naoTimesdev/showtimes-rs/blob/master/LICENSE-MPL) license. Anyone is free to use and redistribute this project and make sure to link back to the original project. More info: [GNU Affero General Public License v3](https://www.tldrlegal.com/license/gnu-affero-general-public-license-v3-agpl-3-0)
