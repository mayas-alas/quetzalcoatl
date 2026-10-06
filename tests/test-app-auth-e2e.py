#!/usr/bin/env python3
"""Real pinned OAuth2 Proxy/Valkey with an isolated, signed test OIDC issuer.

Never touches Pocket users or the managed runtime. This is protocol acceptance,
NOT a personal Pocket/passkey login. Run in host-user development WSL as root.
Protected persistent validation state; no temporary files or secrets in output.
"""
import base64
import hashlib
import http.client
import http.cookies
import json
import os
from pathlib import Path
import secrets
import ssl
import subprocess
import threading
import time
from http.server import BaseHTTPRequestHandler, ThreadingHTTPServer
from urllib.parse import parse_qs, urlencode, urlsplit

ROOT = Path(__file__).resolve().parents[1]
BASE = Path('/var/lib/gnx-build-validation/app-auth-e2e')
ISSUER = 'https://test-idp.gnx:19443'
APP = 'https://test-app.gnx:18443'
PROXY = 'gnx-validation-app-auth'
REDIS = 'gnx-validation-app-sessions'
GROUPS = ['gnx-app-users']
# Pocket passkey identities need not have a verified mailbox.
EMAIL_VERIFIED = False
BAD_NONCE = False
RUN_ID = secrets.token_hex(12)
CODES = {}
REFRESHES = {}
CERT = BASE / 'test-cert.pem'
KEY = BASE / 'test-key.pem'


def command(*args, **kwargs):
    return subprocess.run(args, check=True, stdout=subprocess.PIPE, stderr=subprocess.PIPE, **kwargs).stdout


def encoded(data):
    return base64.urlsafe_b64encode(data).rstrip(b'=').decode()


def store(name, data):
    path = BASE / name
    with path.open('wb') as f:
        f.write(data if isinstance(data, bytes) else data.encode())
    os.chown(path, 0, 10001)
    os.chmod(path, 0o640)
    return path


def jwt(claims):
    data = (encoded(json.dumps({'alg': 'RS256', 'kid': 'test-key'}).encode()) + '.' + encoded(json.dumps(claims).encode())).encode()
    signature = command('openssl', 'dgst', '-sha256', '-sign', str(KEY), input=data)
    return data.decode() + '.' + encoded(signature)


class Server(ThreadingHTTPServer):
    def handle_error(self, request, client_address):
        # Startup probes can race the proxy listener; never emit tracebacks or URLs.
        pass


class Issuer(BaseHTTPRequestHandler):
    def log_message(self, *args):
        pass

    def send(self, code, data=None, location=None):
        self.send_response(code)
        if location:
            self.send_header('Location', location)
        if data is not None:
            data = json.dumps(data).encode()
            self.send_header('Content-Type', 'application/json')
            self.send_header('Content-Length', str(len(data)))
        self.end_headers()
        if data is not None:
            self.wfile.write(data)

    def do_GET(self):
        parsed = urlsplit(self.path)
        if parsed.path == '/.well-known/openid-configuration':
            self.send(200, {'issuer': ISSUER, 'authorization_endpoint': ISSUER + '/authorize',
                           'token_endpoint': ISSUER + '/token', 'userinfo_endpoint': ISSUER + '/userinfo',
                           'jwks_uri': ISSUER + '/jwks', 'response_types_supported': ['code'],
                           'subject_types_supported': ['public'], 'id_token_signing_alg_values_supported': ['RS256'],
                           'token_endpoint_auth_methods_supported': ['client_secret_basic', 'client_secret_post'],
                           'code_challenge_methods_supported': ['S256']})
        elif parsed.path == '/jwks':
            modulus = command('openssl', 'rsa', '-in', str(KEY), '-modulus', '-noout').decode().strip().split('=')[1]
            self.send(200, {'keys': [{'kty': 'RSA', 'kid': 'test-key', 'alg': 'RS256', 'use': 'sig',
                                     'n': encoded(bytes.fromhex(modulus)), 'e': 'AQAB'}]})
        elif parsed.path == '/authorize':
            query = {k: v[0] for k, v in parse_qs(parsed.query).items()}
            if query.get('redirect_uri') != APP + '/oauth2/callback' or query.get('code_challenge_method') != 'S256':
                self.send(400)
                return
            code = secrets.token_urlsafe(24)
            CODES[code] = {**query, 'groups': list(GROUPS)}
            self.send(302, location=APP + '/oauth2/callback?' + urlencode({'code': code, 'state': query['state']}))
        elif parsed.path == '/userinfo':
            self.send(200, {'sub': 'test-member', 'email': 'test-member@example.test', 'email_verified': EMAIL_VERIFIED, 'groups': list(GROUPS)})
        else:
            self.send(404)

    def do_POST(self):
        if self.path != '/token':
            self.send(404)
            return
        body = parse_qs(self.rfile.read(int(self.headers.get('Content-Length', '0'))).decode())
        body = {k: v[0] for k, v in body.items()}
        basic = 'Basic ' + base64.b64encode(('gnx-test-client:' + CLIENT_SECRET).encode()).decode()
        if self.headers.get('Authorization') != basic and body.get('client_secret') != CLIENT_SECRET:
            self.send(401)
            return
        if body.get('grant_type') == 'authorization_code':
            grant = CODES.pop(body.get('code', ''), None)
            verifier = encoded(hashlib.sha256(body.get('code_verifier', '').encode()).digest())
            if not grant or verifier != grant['code_challenge']:
                self.send(400)
                return
            refresh = secrets.token_urlsafe(24)
            REFRESHES[refresh] = grant
        else:
            refresh = body.get('refresh_token', '')
            grant = REFRESHES.get(refresh)
            if not grant:
                self.send(400)
                return
            grant = {**grant, 'groups': list(GROUPS)}
        now = int(time.time())
        claims = {'iss': ISSUER, 'aud': 'gnx-test-client', 'sub': 'test-member',
                  'iat': now, 'exp': now + 300, 'nonce': 'incorrect-test-nonce' if BAD_NONCE else grant['nonce'],
                  'email': 'test-member@example.test', 'email_verified': EMAIL_VERIFIED, 'groups': grant['groups']}
        self.send(200, {'access_token': secrets.token_urlsafe(24), 'id_token': jwt(claims),
                        'token_type': 'Bearer', 'expires_in': 300, 'refresh_token': refresh})


class Frontend(BaseHTTPRequestHandler):
    """Test TLS front only; production Caddy CSRF controls have separate live checks."""
    def log_message(self, *args):
        pass

    def do_GET(self):
        connection = http.client.HTTPConnection('127.0.0.1', 14180, timeout=10)
        connection.request('GET', self.path, headers={'Host': 'test-app.gnx:18443',
                                                    'X-Forwarded-Proto': 'https',
                                                    'Cookie': self.headers.get('Cookie', '')})
        response = connection.getresponse()
        body = response.read()
        self.send_response(response.status)
        for name, value in response.getheaders():
            if name.lower() not in ('transfer-encoding', 'connection', 'content-length'):
                self.send_header(name, value)
        self.send_header('Content-Length', str(len(body)))
        self.end_headers()
        self.wfile.write(body)
        connection.close()


class Browser:
    def __init__(self):
        self.cookies = {}
        self.cookie_properties = {}

    def request(self, url):
        parsed = urlsplit(url)
        context = ssl.create_default_context(cafile=str(CERT))
        connection = http.client.HTTPSConnection('127.0.0.1', parsed.port, timeout=10, context=context)
        import socket
        connection.sock = context.wrap_socket(socket.create_connection(('127.0.0.1', parsed.port), timeout=10), server_hostname=parsed.hostname)
        headers = {'Host': parsed.netloc}
        if parsed.hostname == 'test-app.gnx':
            headers['Cookie'] = '; '.join(f'{k}={v}' for k, v in self.cookies.items())
        connection.request('GET', parsed.path + ('?' + parsed.query if parsed.query else ''), headers=headers)
        response = connection.getresponse()
        result = response.status, response.getheaders(), response.read()
        if parsed.hostname == 'test-app.gnx':
            for name, value in result[1]:
                if name.lower() == 'set-cookie':
                    jar = http.cookies.SimpleCookie(value)
                    for key, cookie in jar.items():
                        self.cookie_properties[key] = dict(cookie.items())
                        if cookie.value:
                            self.cookies[key] = cookie.value
                        else:
                            self.cookies.pop(key, None)
        connection.close()
        return result

    def login(self):
        location = APP + '/oauth2/start?rd=%2F'
        for _ in range(5):
            status, headers, body = self.request(location)
            if status != 302:
                return status, headers, body
            location = next(value for name, value in headers if name.lower() == 'location')
            if location.startswith('/'):
                location = APP + location
        raise AssertionError('redirect loop')


def check(label, passed):
    if not passed:
        raise AssertionError(label)
    print(json.dumps({'test': label, 'result': 'pass', 'issuer': 'isolated-test-not-Pocket-personal'}), flush=True)


if __name__ == '__main__':
    if os.geteuid() != 0:
        raise SystemExit('ROOT_REQUIRED_FOR_ISOLATED_VALIDATION')
    servers = []
    try:
        BASE.mkdir(mode=0o750, parents=True, exist_ok=True)
        os.chown(BASE, 0, 10001)
        os.chmod(BASE, 0o750)
        if not CERT.exists():
            command('openssl', 'req', '-x509', '-newkey', 'rsa:2048', '-nodes', '-days', '30',
                    '-keyout', str(KEY), '-out', str(CERT), '-subj', '/CN=GnX isolated auth test',
                    '-addext', 'subjectAltName=DNS:test-idp.gnx,DNS:test-app.gnx')
        os.chown(KEY, 0, 10001)
        os.chmod(KEY, 0o640)
        CLIENT_SECRET = secrets.token_urlsafe(32)
        REDIS_SECRET = secrets.token_hex(32)
        store('client.secret', CLIENT_SECRET)
        store('cookie.secret', secrets.token_bytes(32))
        store('sessions.conf', 'bind 127.0.0.1\nport 16380\nprotected-mode yes\nsave ""\nappendonly no\nrequirepass ' + REDIS_SECRET + '\n')
        web = BASE / 'web'
        web.mkdir(exist_ok=True)
        os.chmod(web, 0o755)
        (web / 'index.html').write_text('GNX_ISOLATED_PRIVATE_MARKER')
        os.chmod(web / 'index.html', 0o644)
        config = (ROOT / 'payload/node/auth/oauth2-proxy.cfg').read_text()
        config = config.replace('https://pocket.gnx', ISSUER).replace('https://app.gnx/oauth2/callback', APP + '/oauth2/callback')
        config = config.replace('/etc/gnx/ca/gateway-ca.pem', '/validation/test-cert.pem')
        config = config.replace('/etc/gnx/auth/', '/validation/').replace('/srv/gnx-web/', '/validation/web/')
        config = config.replace('127.0.0.1:4180', '127.0.0.1:14180').replace('127.0.0.1:6380', '127.0.0.1:16380')
        config = config.replace('cookie_refresh = "1m"', 'cookie_refresh = "1s"').replace('["app.gnx"]', '["test-app.gnx:18443"]')
        config += '\nclient_id = "gnx-test-client"\nredis_password = ' + json.dumps(REDIS_SECRET) + '\n'
        store('oauth2-proxy.cfg', config)
        for port, handler in [(19443, Issuer), (18443, Frontend)]:
            server = Server(('127.0.0.1', port), handler)
            ctx = ssl.SSLContext(ssl.PROTOCOL_TLS_SERVER)
            ctx.load_cert_chain(str(CERT), str(KEY))
            server.socket = ctx.wrap_socket(server.socket, server_side=True)
            threading.Thread(target=server.serve_forever, daemon=True).start()
            servers.append(server)
        redis_ref = next(line.split('=', 1)[1] for line in (ROOT / 'payload/node/services/app-sessions.container').read_text().splitlines() if line.startswith('Image='))
        proxy_ref = next(line.split('=', 1)[1] for line in (ROOT / 'payload/node/services/app-auth.container').read_text().splitlines() if line.startswith('Image='))
        command('podman', 'run', '-d', '--pull=never', '--name', REDIS, '--network=host', '--user', '10001:10001',
                '--cap-drop=all', '--security-opt=no-new-privileges', '--read-only', '--label=io.gnx.validation=' + RUN_ID,
                '-v', str(BASE) + ':/validation:ro',
                redis_ref, 'valkey-server', '/validation/sessions.conf')
        command('podman', 'run', '-d', '--pull=never', '--name', PROXY, '--network=host', '--user', '10001:10001',
                '--cap-drop=all', '--security-opt=no-new-privileges', '--read-only', '--label=io.gnx.validation=' + RUN_ID,
                '--add-host=test-idp.gnx:127.0.0.1',
                '-v', str(BASE) + ':/validation:ro', proxy_ref, '--config=/validation/oauth2-proxy.cfg')
        for _ in range(30):
            try:
                anonymous = Browser().request(APP + '/')
                break
            except OSError:
                time.sleep(0.5)
        else:
            raise AssertionError('proxy startup')
        check('anonymous_denied', anonymous[0] == 302 and b'GNX_ISOLATED_PRIVATE_MARKER' not in anonymous[2])
        member = Browser()
        status, headers, body = member.login()
        check('signed_oidc_member_access', status == 200 and body == b'GNX_ISOLATED_PRIVATE_MARKER')
        check('passkey_identity_with_unverified_email_access', not EMAIL_VERIFIED and status == 200)
        session_name = '__Host-GnX-Session'
        check('session_cookie_present', session_name in member.cookies)
        props = member.cookie_properties[session_name]
        check('secure_httponly_host_cookie', props['secure'] and props['httponly'] and props['samesite'].lower() == 'lax'
              and props['path'] == '/' and not props['domain'])
        forged = Browser()
        forged.cookies = {session_name: 'forged-ticket'}
        check('forged_session_denied', forged.request(APP + '/')[0] == 302)
        csrf_browser = Browser()
        _, headers, _ = csrf_browser.request(APP + '/oauth2/start?rd=%2F')
        authorize = next(v for k, v in headers if k.lower() == 'location')
        _, headers, _ = csrf_browser.request(authorize)
        callback = next(v for k, v in headers if k.lower() == 'location')
        status, _, body = Browser().request(callback)
        check('callback_without_csrf_denied', status == 403 and b'GNX_ISOLATED_PRIVATE_MARKER' not in body)
        replay = Browser()
        replay.cookies = dict(member.cookies)
        status, headers, body = member.request(APP + '/oauth2/sign_out?rd=' + APP + '/signed-out')
        check('logout_cookie_deleted', status == 302 and session_name not in member.cookies)
        status, headers, body = replay.request(APP + '/')
        check('logout_server_session_replay_denied', status == 302 and b'GNX_ISOLATED_PRIVATE_MARKER' not in body)
        BAD_NONCE = True
        status, headers, body = Browser().login()
        check('signed_token_with_wrong_nonce_denied', status >= 400 and b'GNX_ISOLATED_PRIVATE_MARKER' not in body)
        BAD_NONCE = False
        GROUPS = []
        status, headers, body = Browser().login()
        check('signed_oidc_without_group_denied', status == 403 and b'GNX_ISOLATED_PRIVATE_MARKER' not in body)
        GROUPS = ['gnx-app-users']
        member = Browser()
        check('member_relogin', member.login()[0] == 200)
        GROUPS = []
        time.sleep(2)
        status, headers, body = member.request(APP + '/')
        check('refreshed_group_removal_denied', status in (302, 403) and b'GNX_ISOLATED_PRIVATE_MARKER' not in body)
        check('no_auth_tokens_in_application_body', b'access_token' not in body and b'id_token' not in body)
        GROUPS = ['gnx-app-users']
        member = Browser()
        check('member_before_session_store_failure', member.login()[0] == 200)
        command('podman', 'stop', REDIS)
        status, headers, body = member.request(APP + '/')
        check('session_store_outage_fails_closed', status in (302, 403, 500, 503) and b'GNX_ISOLATED_PRIVATE_MARKER' not in body)
    except Exception as error:
        # No HTTP URLs, headers, claims, cookies, bodies or process logs in errors.
        print(json.dumps({'test': str(error) if isinstance(error, AssertionError) else 'isolated_auth_runtime',
                          'result': 'fail', 'exception_type': type(error).__name__}), flush=True)
        raise SystemExit(1)
    finally:
        for name in (PROXY, REDIS):
            ownership = subprocess.run(['podman', 'inspect', '--format', '{{index .Config.Labels "io.gnx.validation"}}', name],
                                       stdout=subprocess.PIPE, stderr=subprocess.DEVNULL)
            if ownership.stdout.decode().strip() == RUN_ID:
                subprocess.run(['podman', 'rm', '-f', name], stdout=subprocess.DEVNULL, stderr=subprocess.DEVNULL)
        for server in servers:
            server.shutdown()
