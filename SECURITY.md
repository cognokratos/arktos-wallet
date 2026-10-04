# Security Policy

Arktos Wallet handles wallet secrets (recovery phrases, private keys, API keys).
Please treat any vulnerability in it as potentially fund-affecting.

## Reporting a vulnerability

**Do not open a public issue, pull request, or discussion for security problems.**

Report vulnerabilities privately through GitHub Security Advisories:

1. Go to the repository's **Security** tab.
2. Choose **Report a vulnerability**
   (direct link: <https://github.com/cognokratos/arktos-wallet/security/advisories/new>).
3. Include a description, affected version/commit, reproduction steps, and the
   impact you expect (e.g. key disclosure, authentication bypass, fund loss).

Please give the maintainers reasonable time to investigate and release a fix
before any public disclosure. We will coordinate the disclosure timeline and
credit with you through the advisory.

## Supported versions

The project is pre-1.0 and has no tagged releases yet. Security fixes are made on:

| Branch | Supported |
|--------|-----------|
| `main` | ✅ |
| `dev`  | ✅ |
| anything else | ❌ |

## Known issues

Accepted dependency advisories are listed, with justification, in
[`deny.toml`](./deny.toml) and [`.cargo/audit.toml`](./.cargo/audit.toml).

## Disclaimer

Arktos Wallet is an educational blueprint. Review and harden it for your own
threat model before handling real funds.
