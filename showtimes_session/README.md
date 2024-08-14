# session

<a href="https://jwt.io/"><img src="https://jwt.io/img/logo-asset.svg" alt="JWT Compatible" height="50"></a>

The session handling library for Showtimes API. Powered by [JWT](https://jwt.io/).

This library contains four main functions:
- `create_*`: Create a new session.
  - `create_session`: Create a new session for user authentication.
  - `create_discord_session_state`: Create a anti-CSRF state for Discord OAuth2.
- `verify_*`: Verify a session.
  - `verify_session`: Verify a session for user authentication.
  - `verify_discord_session_state`: Verify a anti-CSRF state for Discord OAuth2.

We use `HS512` for the algorithm that contains the following payload:
```jsonc
{
    // The token issue date
    "iat": 1620000000,
    // The token expiration date
    "exp": 1620000000,
    // The token issuer
    "iss": "showtimes-rs-session/0.1.0",
    // The token "audience", `user` or `discord-auth` to determine the token usage
    "aud": "user",
    // The user ULID or the final redirect URL for Discord
    "metadata": "1234567890"
}
```

The verification process will check the following:
- The token issuer must be `showtimes-rs-session/0.1.0`.
- The token expiration date must be greater than the current time.
- The token audience must be `user` or `discord-auth`.
- The token metadata must be a valid ULID or a valid URL (this will be done on another crates).
