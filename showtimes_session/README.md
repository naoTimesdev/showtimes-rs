# session

<a href="https://jwt.io/"><img src="https://jwt.io/img/logo-asset.svg" alt="JWT Compatible" height="50"></a>

The session handling library for Showtimes API. Powered by [JWT](https://jwt.io/).

This library contains four main functions:
- `create_*`: Create a new session.
  - `create_session`: Create a new session for JWT authentication.
  - `create_api_key_session`: Create a new session for API key authentication.
  - `create_discord_session_state`: Create a anti-CSRF state for Discord OAuth2.
- `verify_session`: Verify a session token with the expected audience.

We support the following algorithm:
- `HS256`, `HS384`, `HS512`
- `PS256`, `PS384`, `PS512` (We do not support `RS256`, `RS384`, `RS512`)
- `ES256`, `ES384`

You can create the specific algorithm with the `ShowtimesEncodingKey::new_*` function.

The following is the example payload:
```jsonc
{
    // The token issue date
    "iat": 1620000000,
    // The token expiration date
    "exp": 1620000000,
    // The token issuer
    "iss": "showtimes-rs-session/0.1.0",
    // The token "audience": `user`, "api-key", "master-key", or "discord-auth".
    "aud": "user",
    // The user ULID, API key, or the final redirect URL for Discord
    "metadata": "1234567890"
}
```

The verification process will check the following:
- The token issuer must be `showtimes-rs-session/0.1.0`.
- The token expiration date must be greater than the current time.
- The token audience must be `user` or `discord-auth`.
- The token metadata must be a valid ULID or a valid URL (this will be done on another crates).

## License

This crates has been licensed under the [MPL 2.0](https://github.com/naoTimesdev/showtimes-rs/blob/master/LICENSE-MPL) license. Anyone is free to use and redistribute this project and make sure to link back to the original project. More info: [Mozilla Public License 2.0](https://www.tldrlegal.com/license/mozilla-public-license-2-0-mpl-2)
