#!/usr/bin/env python3
"""BYOP account/custom-domain contracts with in-memory API/state fixtures.
No runtime mutation, network traffic, credentials or temporary files.
"""
import importlib.machinery
import importlib.util
import io
import json
from pathlib import Path
import stat
from types import SimpleNamespace
import unittest
import urllib.error
from unittest.mock import patch

SOURCE = Path(__file__).resolve().parents[1] / 'payload/node/workloads/ai/proxy-enroll.run'
loader = importlib.machinery.SourceFileLoader('ai_proxy_enrollment', str(SOURCE))
spec = importlib.util.spec_from_loader(loader.name, loader)
module = importlib.util.module_from_spec(spec)
loader.exec_module(module)


class MemoryPath:
    def __init__(self, state, name=''):
        self.state, self.name = state, name
    def __truediv__(self, name):
        return MemoryPath(self.state, name)
    def exists(self):
        return self.name in self.state


class EnrollmentTests(unittest.TestCase):
    def setUp(self):
        self.cfg = {'cluster_domain': 'proxy.example.test', 'custom_domain': 'apps.example.test'}
        self.files = {}
        self.tokens, self.domains = [], []
        self.cluster = {'address': self.cfg['cluster_domain'], 'type': 'account', 'online': True, 'connected_proxies': 1}
        self.calls = []
        self.patches = [
            patch.object(module, 'BASE', MemoryPath(self.files)),
            patch.object(module, 'private', side_effect=lambda p: self.files[p.name]),
            patch.object(module, 'persist', side_effect=self.persist),
            patch.object(module, 'request', side_effect=self.api),
            patch.object(module.time, 'sleep'),
        ]
        for p in self.patches:
            p.start()
            self.addCleanup(p.stop)
    def persist(self, path, value):
        if path.name in self.files:
            raise FileExistsError('creation only')
        self.files[path.name] = value
    def api(self, method, path, pat, body=None):
        self.assertEqual(pat, 'test-pat-not-a-credential')
        self.calls.append((method, path, body))
        if path == 'proxy-tokens' and method == 'GET':
            return self.tokens
        if path == 'proxy-tokens' and method == 'POST':
            token = {'id': 'token-id', 'name': body['name'], 'revoked': False}
            self.tokens.append(token)
            return {**token, 'plain_token': 'nbx_fixture_not_a_real_token'}
        if path == 'clusters':
            return [self.cluster]
        if path == 'domains' and method == 'GET':
            return self.domains
        if path == 'domains' and method == 'POST':
            result = {'id': 'domain-id', 'type': 'custom', 'validated': True, **body}
            self.domains.append(result)
            return result
        if path == 'domains/domain-id/validate':
            return None
        raise AssertionError('unexpected API call')
    def call(self, action):
        return getattr(module, action)('test-pat-not-a-credential', self.cfg)
    def test_account_token_creation_and_idempotent_retry(self):
        self.call('token')
        state = dict(self.files)
        self.call('token')
        self.assertEqual(self.files, state)
        posts = [c for c in self.calls if c[:2] == ('POST', 'proxy-tokens')]
        self.assertEqual(len(posts), 1)
        self.assertEqual(posts[0][2]['expires_in'], 0)
        self.assertNotIn('plain_token', self.files['ai-proxy-token.json'])
    def test_changed_cluster_does_not_overwrite_identity(self):
        self.call('token')
        original = dict(self.files)
        self.cfg['cluster_domain'] = 'other.example.test'
        with self.assertRaisesRegex(module.Blocked, 'OWNERSHIP_CONFLICT'):
            self.call('token')
        self.assertEqual(self.files, original)
    def test_revoked_token_is_not_silently_rotated(self):
        self.call('token')
        self.tokens[0]['revoked'] = True
        with self.assertRaisesRegex(module.Blocked, 'TOKEN_REVOKED_OR_MISSING'):
            self.call('token')
        self.assertEqual(sum(c[0] == 'POST' for c in self.calls), 1)
    def test_interrupted_receipt_requires_explicit_recovery(self):
        self.call('token')
        del self.files['ai-proxy-token.json']
        with self.assertRaisesRegex(module.Blocked, 'TOKEN_RECOVERY_REQUIRED'):
            self.call('token')
        self.assertEqual(sum(c[0] == 'POST' for c in self.calls), 1)
    def test_management_wide_shared_cluster_is_not_accepted(self):
        self.cluster['type'] = 'shared'
        with self.assertRaisesRegex(module.Blocked, 'ACCOUNT_CLUSTER_REQUIRED'):
            self.call('domains')
        self.assertFalse(any(c[0] == 'POST' for c in self.calls))
    def test_offline_cluster_is_not_ready(self):
        for change in ({'online': False}, {'connected_proxies': 0}):
            self.cluster.update(change)
            with self.assertRaisesRegex(module.Blocked, 'CLUSTER_OFFLINE'):
                self.call('domains')
    def test_custom_domain_registration_and_verified_cluster_binding(self):
        self.call('domains')
        self.call('domains')
        self.call('verify')
        self.assertEqual(sum(c[:2] == ('POST', 'domains') for c in self.calls), 1)
        self.domains[0]['target_cluster'] = 'other.example.test'
        with self.assertRaisesRegex(module.Blocked, 'DOMAIN_VERIFICATION_REQUIRED'):
            self.call('verify')
    def test_unverified_domain_is_not_ready(self):
        self.call('domains')
        self.domains[0]['validated'] = False
        with self.assertRaisesRegex(module.Blocked, 'DOMAIN_VERIFICATION_REQUIRED'):
            self.call('verify')
    def test_existing_unowned_domain_is_not_adopted(self):
        self.domains.append({'domain': self.cfg['custom_domain']})
        with self.assertRaisesRegex(module.Blocked, 'DOMAIN_UNOWNED'):
            self.call('domains')
        self.assertFalse(any(c[0] == 'POST' for c in self.calls))
    def test_domain_validation_rejects_urls_wildcards_credentials_and_ips(self):
        for value in ('https://example.test', '*.example.test', 'x:secret@example.test',
                      '127.0.0.1', 'localhost', '-x.example.test', 'a..example.test', 'x.example.test/path'):
            with self.subTest(value=value), self.assertRaises(module.Blocked):
                module.domain(value)
        self.assertEqual(module.domain('Proxy.Example.Test.'), 'proxy.example.test')
    def test_http_failure_does_not_include_response_body_or_credential(self):
        self.patches[3].stop()
        error = urllib.error.HTTPError('http://localhost/api', 403, 'denied', {}, io.BytesIO(b'sensitive-response-body'))
        with patch.object(module.urllib.request, 'urlopen', side_effect=error):
            with self.assertRaises(module.Blocked) as caught:
                module.request('GET', 'clusters', 'test-pat-not-a-credential')
        self.assertEqual(str(caught.exception), 'AI_PROXY_API_HTTP_403')
        self.assertNotIn('sensitive-response-body', str(caught.exception))
        self.assertNotIn('test-pat-not-a-credential', str(caught.exception))
    def test_private_file_rejects_world_readable_nonroot_and_symlinks(self):
        self.patches[1].stop()
        for mode, uid in ((stat.S_IFREG | 0o644, 0), (stat.S_IFREG | 0o600, 1000), (stat.S_IFLNK | 0o600, 0)):
            path = SimpleNamespace(lstat=lambda m=mode, u=uid: SimpleNamespace(st_mode=m, st_uid=u))
            with self.assertRaisesRegex(module.Blocked, 'SECRET_PERMISSIONS'):
                module.private(path)
    def test_no_management_wide_token_or_certificate_bypass(self):
        text = SOURCE.read_text()
        self.assertNotIn('skip_tls_verify', text)
        self.assertNotIn('NB_PROXY_ALLOW_INSECURE', text)
        self.assertNotIn('authSecret', text)
        self.assertEqual(module.API, 'http://127.0.0.1:8081/api/reverse-proxies/')


if __name__ == '__main__':
    unittest.main(verbosity=2)
