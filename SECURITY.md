# Security Policy

## Reporting a Vulnerability

If you discover a security vulnerability in GlobalTelco, please report it responsibly:

1. **Do NOT open a public GitHub issue** for security vulnerabilities
2. Email or DM the maintainer directly via [GitHub](https://github.com/KodyDennon)
3. Include a description of the vulnerability and steps to reproduce

You should receive a response within 48 hours. We take security seriously and will work to address valid reports promptly.

## Scope

This policy applies to:

- The GlobalTelco game server (`gt-server`)
- The WASM simulation module (`gt-wasm`)
- The web frontend (`web/`)
- The multiplayer WebSocket protocol
- Authentication and admin endpoints

## Known Security Considerations

- The multiplayer server implements per-type rate limiting, spatial validation, and sequence number deduplication
- Admin endpoints require an `ADMIN_KEY` environment variable
- JWT-based authentication with configurable token expiry
- WebSocket connections are validated on join
