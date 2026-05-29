# Local HTTPS certificates

This directory is a placeholder for development-only localhost TLS certificates.

Do not commit generated certificate or key files. `web/https_server.py` creates
`localhost-cert.pem` and `localhost-key.pem` under `~/.cache/rhwp/certs` by
default. Set `RHWP_CERT_DIR` only if you need a custom local path.
