#!/usr/bin/env python3
"""Memory-only tests; do not query a tailnet or edit its policy."""
import ast
import importlib.machinery
import types
import sys
from pathlib import Path
import unittest

sys.dont_write_bytecode = True
path = Path(__file__).resolve().parent.parent / 'payload/node/compute/access-mesh-policy.run'
module = types.ModuleType('access_mesh_policy')
importlib.machinery.SourceFileLoader(module.__name__, str(path)).exec_module(module)


class PolicyTests(unittest.TestCase):
    source = '100.67.195.81'
    destination = '100.127.95.60'

    def rule(self, first=443, last=443, protocol=6):
        return [{'SrcIPs': [self.source + '/32'], 'DstPorts': [{'IP': self.destination + '/32', 'Ports': {'First': first, 'Last': last}}], 'IPProto': [protocol]}]

    def test_known_https_does_not_authorize_extra_ports(self):
        self.assertTrue(module.permits(self.rule(), self.source, self.destination, 443, 6))
        for port in (1411, 8080, 8081):
            self.assertFalse(module.permits(self.rule(), self.source, self.destination, port, 6))

    def test_udp_is_not_tcp(self):
        self.assertTrue(module.permits(self.rule(3478, 3478, 17), self.source, self.destination, 3478, 17))
        self.assertFalse(module.permits(self.rule(3478, 3478, 6), self.source, self.destination, 3478, 17))

    def test_source_and_destination_are_both_scoped(self):
        self.assertFalse(module.permits(self.rule(), '100.67.195.82', self.destination, 443, 6))
        self.assertFalse(module.permits(self.rule(), self.source, '100.127.95.61', 443, 6))

    def test_empty_policy_denies(self):
        self.assertFalse(module.permits([], self.source, self.destination, 443, 6))

    def test_resolved_wildcard_and_legacy_protocol(self):
        rules = [{'SrcIPs': ['*'], 'DstPorts': [{'IP': '*', 'Ports': {'First': 0, 'Last': 65535}}]}]
        self.assertTrue(module.permits(rules, self.source, self.destination, 3478, 17))


verify = path.parent / 'access-verify.run'
function = next(node for node in ast.parse(verify.read_text()).body
                if isinstance(node, ast.FunctionDef) and node.name == 'grpc_response_valid')
namespace = {}
exec(compile(ast.Module(body=[function], type_ignores=[]), str(verify), 'exec'), namespace)


class GrpcHeaderTests(unittest.TestCase):
    def test_http2_status_whitespace_and_trailers(self):
        for status in (b'HTTP/2 200\r\n', b'HTTP/2 200 \r\n'):
            headers = status + b'content-type: application/grpc\r\n\r\ngrpc-status: 0\r\n'
            self.assertTrue(namespace['grpc_response_valid'](headers, '0'))

    def test_rejects_ui_http1_errors_and_missing_trailers(self):
        for headers in (
            b'HTTP/1.1 200 OK\r\ncontent-type: application/grpc\r\ngrpc-status: 0\r\n',
            b'HTTP/2 200 \r\ncontent-type: text/html\r\ngrpc-status: 0\r\n',
            b'HTTP/2 200 \r\ncontent-type: application/grpc\r\ngrpc-status: 13\r\n',
            b'HTTP/2 200 \r\ncontent-type: application/grpc\r\n',
        ):
            self.assertFalse(namespace['grpc_response_valid'](headers, '0'))


if __name__ == '__main__':
    unittest.main()
