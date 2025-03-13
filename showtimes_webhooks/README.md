# webhooks

Webhooks delivery system for Showtimes.

Currently supported webhooks:
- Discord

Available webhook type:
- `new-project`: When a new project is added to the database.
- `project-progress`: When a project progress is updated.
- `project-release`: When one of the "episode" is released.
- `project-dropped`: When a project is dropped.
- `project-resumed`: When a project is resumed.

## License

This crates has been licensed under the [MPL 2.0](https://github.com/naoTimesdev/showtimes-rs/blob/master/LICENSE-MPL) license. Anyone is free to use and redistribute this project and make sure to link back to the original project. More info: [Mozilla Public License 2.0](https://www.tldrlegal.com/license/mozilla-public-license-2-0-mpl-2)
