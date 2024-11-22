# gql

GraphQL models, interactor, and many more.

## Structure
The gql crates is split into more parts to make it easier to be compiled and managed:
- `common` — contains some more common model and some helpers like data loader and other
- `models` — contains the model itself that is mapped from the database
  - `events` — event log specific model from ClickHouse
- `paginator` — the paginator action that handles the pagination logic for some queries action
- `queries` — the actual queries model
- `mutations` — the actual mutations model
- `subscriptions` — the actual subscriptions/watch model

## License

All sub-crates has been licensed under the [AGPL 3.0 only](https://github.com/naoTimesdev/showtimes-rs/blob/master/LICENSE-AGPL) license. Anyone is free to use and redistribute this project and make sure to link back to the original project. More info: [GNU Affero General Public License v3](https://www.tldrlegal.com/license/gnu-affero-general-public-license-v3-agpl-3-0)