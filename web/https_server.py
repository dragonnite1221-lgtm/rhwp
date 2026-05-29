#!/usr/bin/env python3
"""로컬 HTTPS 개발 서버 (Clipboard API text/html 읽기용)"""
import http.server
import os
import shutil
import ssl
import subprocess
import sys

PORT = int(sys.argv[1]) if len(sys.argv) > 1 else 7700
BIND = '0.0.0.0'
DEFAULT_CERT_DIR = os.path.join(os.path.expanduser('~'), '.cache', 'rhwp', 'certs')
CERT_DIR = os.environ.get('RHWP_CERT_DIR', DEFAULT_CERT_DIR)
CERTFILE = os.path.join(CERT_DIR, 'localhost-cert.pem')
KEYFILE = os.path.join(CERT_DIR, 'localhost-key.pem')


def ensure_local_cert():
    if os.path.exists(CERTFILE) and os.path.exists(KEYFILE):
        return

    openssl = shutil.which('openssl')
    if openssl is None:
        raise SystemExit(
            '로컬 HTTPS 인증서 생성에 필요한 openssl을 설치한 뒤 다시 실행하세요.'
        )

    os.makedirs(CERT_DIR, exist_ok=True)
    subprocess.run(
        [
            openssl,
            'req',
            '-x509',
            '-newkey',
            'rsa:2048',
            '-sha256',
            '-days',
            '825',
            '-nodes',
            '-keyout',
            KEYFILE,
            '-out',
            CERTFILE,
            '-subj',
            '/CN=localhost',
            '-addext',
            'subjectAltName=DNS:localhost,IP:127.0.0.1,IP:::1',
        ],
        check=True,
    )
    os.chmod(KEYFILE, 0o600)
    os.chmod(CERTFILE, 0o644)


# 프로젝트 루트에서 서빙 (../pkg/ 경로 접근 가능하도록)
os.chdir(os.path.join(os.path.dirname(os.path.abspath(__file__)), '..'))

ensure_local_cert()

context = ssl.SSLContext(ssl.PROTOCOL_TLS_SERVER)
context.minimum_version = ssl.TLSVersion.TLSv1_2
context.load_cert_chain(CERTFILE, KEYFILE)

server = http.server.HTTPServer((BIND, PORT), http.server.SimpleHTTPRequestHandler)
server.socket = context.wrap_socket(server.socket, server_side=True)

print(f'HTTPS 서버 시작: https://localhost:{PORT}/web/editor.html')
server.serve_forever()
