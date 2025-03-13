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
- `ES256`, `ES384`, `ES512`
- `EdDSA`, `ES256K`

The JWT engine is powered by [`jwt-lc-rs`](https://github.com/noaione/jwt-lc-rs),
it uses AWS-LC FIPS compliant crypto library to provide the best security and performance.

The following is the example payload:
```jsonc
{
    // The token issue date
    "iat": 1620000000,
    // The token expiration date
    "exp": 1620000000,
    // The token issuer
    "iss": "naoTimes/showtimes-rs",
    // The token "audience": `user`, "api-key", "master-key", or "discord-auth".
    "aud": "user",
    // The user ULID, API key, or the final redirect URL for Discord
    "metadata": "1234567890"
}
```

The verification process will check the following:
- The token issuer must be `naoTimes/showtimes-rs`.
- The token expiration date must be greater than the current time.
- The token audience must be `user` or `discord-auth`.
- The token metadata must be a valid ULID or a valid URL (this will be done on another crates).

## Generating PEM

### RSA
For RSA, you can do the following:
```bash
openssl genpkey -algorithm RSA-PSS -out private_key.pem -pkeyopt rsa_keygen_bits:4096 -pkeyopt rsa_pss_keygen_md:sha384 -pkeyopt rsa_pss_keygen_mgf1_md:sha384 -pkeyopt rsa_pss_keygen_saltlen:32 -out ./keys/priv.key
```

This will generate a private key with the following properties:
- 4096 bits
- SHA-384
- MGF1 SHA-384
- Salt length: 32 bytes
- PSS padding

You can adjust it to your needs, the above is the recommended settings.

Then generate the public key with the following command:
```bash
openssl rsa -pubout -in ./keys/priv.key -out ./keys/pub.pem
```

### ECDSA/secp256k1
For ECDSA, you can do the following:
```bash
openssl genpkey -algorithm EC -pkeyopt ec_paramgen_curve:P-256 -out ./keys/priv.key
```

You can change the curve to either:
- `ES256` (`ec_paramgen_curve:P-256`), SHA-256
- `ES384` (`ec_paramgen_curve:P-384`), SHA-384
- `ES512` (`ec_paramgen_curve:P-521`), SHA-512
- `ES256K` (`ec_paramgen_curve:secp256k1`)

**Note**: You need to set the SHA level to be the same as your NIST curve.<br />
For `ES256K`, you need to set the `mode` to `es256k1`.

Then generate the public key with the following command:
```bash
openssl pkey -in ./keys/priv.key -pubout -out ./keys/pub.pem
```

### EdDSA
For EdDSA, you can do the following:
```bash
openssl genpkey -algorithm ED25519 -out ./keys/priv.key
```

This will generate a private key with the following properties:
- Curve: Ed25519 (Which is the only supported one currently)

Then generate the public key with the following command:
```bash
openssl pkey -in ./keys/priv.key -pubout -out ./keys/pub.pem
```

## License

This crates has been licensed under the [MPL 2.0](https://github.com/naoTimesdev/showtimes-rs/blob/master/LICENSE-MPL) license. Anyone is free to use and redistribute this project and make sure to link back to the original project. More info: [Mozilla Public License 2.0](https://www.tldrlegal.com/license/mozilla-public-license-2-0-mpl-2)
