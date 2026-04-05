# Security Policy

## Supported Versions

| Version | Supported |
|---------|-----------|
| 0.1.x   | Yes       |

## Reporting a Vulnerability

If you discover a security vulnerability, please report it responsibly:

1. **Do not** open a public GitHub issue
2. Use [GitHub Security Advisories](https://github.com/cntm-labs/sentinel-driver/security/advisories/new) to report privately
3. Or email: security@cntm-labs.dev

We will acknowledge receipt within 48 hours and aim to provide a fix within 7 days for critical issues.

## Scope

This covers vulnerabilities in:
- Authentication handling (SCRAM-SHA-256, MD5)
- TLS/SSL implementation
- SQL injection through the driver API
- Memory safety issues
- Connection pool security
