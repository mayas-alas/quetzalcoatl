#!/usr/bin/env python3
"""Parent-checked publication from authenticated HOST-USER WSL, never runtime.
Requires huggingface_hub in the operator's Python. Does not create temp files,
rewrite history, upload a standalone MSI or publish ignored offline inputs.
Artifact mode verifies the public download before writing source metadata.
Source mode publishes a clean committed Git snapshot and compares every blob.
"""
import argparse
import hashlib
import io
import json
import os
from pathlib import Path
import re
import subprocess
import urllib.request

from huggingface_hub import HfApi, CommitOperationAdd, CommitOperationDelete

ROOT = Path(__file__).resolve().parents[2]
DATASET = 'gnx-labs-mx/gnx-node-releases'
SPACE = 'gnx-labs-mx/quetzalcoatl'


def git(*args):
    # DrvFS working files use Windows Git's CRLF policy and non-POSIX mode bits.
    # Publish HEAD blobs; do not mistake cross-OS stat/EOL differences for edits.
    return subprocess.run(['git', '-c', 'core.autocrlf=true', '-c', 'core.fileMode=false',
                           '-C', str(ROOT), *args], check=True, stdout=subprocess.PIPE, stderr=subprocess.PIPE).stdout


def hash_file(path):
    digest = hashlib.sha256()
    with path.open('rb') as f:
        for block in iter(lambda: f.read(1024 * 1024), b''):
            digest.update(block)
    return digest.hexdigest()


def tree(api, repo, kind):
    return {entry.path: entry for entry in api.list_repo_tree(repo, repo_type=kind, recursive=True) if hasattr(entry, 'blob_id')}


def attr(obj, name):
    return obj.get(name) if isinstance(obj, dict) else getattr(obj, name, None)


def artifact(api, version):
    exe = ROOT / 'build/installer/GnxNodeSetup.exe'
    checksum, size = hash_file(exe), exe.stat().st_size
    path = f'candidates/{version}/GnxNodeSetup.exe'
    head = api.repo_info(DATASET, repo_type='dataset').sha
    files = tree(api, DATASET, 'dataset')
    if path in files:
        entry = files[path]
        assert attr(entry.lfs, 'sha256') == checksum and entry.size == size, 'IMMUTABLE_ARTIFACT_CHANGED'
        revision = head
    else:
        revision = api.create_commit(DATASET, repo_type='dataset', parent_commit=head,
            commit_message=f'{version} installed availability/security candidate; personal acceptance pending',
            operations=[CommitOperationAdd(path_in_repo=path, path_or_fileobj=str(exe))]).oid
    url = f'https://huggingface.co/datasets/{DATASET}/resolve/{revision}/{path}'
    remote_hash, remote_size = hashlib.sha256(), 0
    with urllib.request.urlopen(url, timeout=120) as response:
        for block in iter(lambda: response.read(1024 * 1024), b''):
            remote_hash.update(block)
            remote_size += len(block)
    assert (remote_hash.hexdigest(), remote_size) == (checksum, size), 'PUBLISHED_DOWNLOAD_MISMATCH'
    release = ROOT / 'releases' / version
    release.mkdir(parents=True, exist_ok=True)
    (release / 'SHA256SUMS.txt').write_text(f'{checksum}  GnxNodeSetup.exe\n')
    (release / 'ARTIFACT.md').write_text(f'''# {version} — candidato instalado, aceptación personal pendiente

[Descargar GnxNodeSetup.exe]({url})

- Dataset revision: `{revision}`.
- Tamaño: **{size} bytes**.
- SHA-256: `{checksum}`.
- Burn contiene MSI/payload offline; no se publica otro MSI.
- Instalado exit 0 y upgrade NodeReady; credenciales preservadas. Descarga
  publicada verificada por streaming contra tamaño/SHA sin archivos temporales.
- **No promovido como aceptación completa**: passkey, grupo y logout/replay
  con una sesión Pocket real pendientes. [Notas](RELEASE_NOTES.md) y
  [evidencia](../../docs/validation/{version}/README.md).
''')
    print(json.dumps({'mode': 'artifact', 'revision': revision, 'size': size, 'sha256': checksum, 'download_verified': True}))


def source(api, expected_head):
    assert not git('status', '--porcelain').strip(), 'SOURCE_MUST_BE_COMMITTED_AND_CLEAN'
    commit = git('rev-parse', 'HEAD').decode().strip()
    head = api.repo_info(SPACE, repo_type='space').sha
    assert head == expected_head, 'SPACE_HEAD_CHANGED_REVIEW_REQUIRED'
    remote = tree(api, SPACE, 'space')
    local, operations = {}, []
    for entry in git('ls-tree', '-r', '-z', '--full-tree', 'HEAD').split(b'\0'):
        if not entry:
            continue
        meta, raw_path = entry.split(b'\t', 1)
        mode, kind, blob = meta.decode().split(' ')
        path = raw_path.decode()
        assert kind == 'blob' and mode in ('100644', '100755'), 'UNSUPPORTED_SOURCE_ENTRY'
        assert not path.endswith(('.exe', '.msi', '/offline.tar.gz')), 'BINARY_INPUT_IN_SOURCE'
        local[path] = blob
        if path in remote and remote[path].blob_id == blob:
            continue
        assert path != 'payload/host/gnx-node-rootfs.tar', 'SEALED_ROOTFS_CHANGED'
        data = git('cat-file', 'blob', blob)
        pointer = re.fullmatch(rb'version https://git-lfs.github.com/spec/v1\noid sha256:([0-9a-f]{64})\nsize ([0-9]+)\n', data)
        if pointer:
            physical = ROOT / path
            assert physical.stat().st_size == int(pointer[2]) and hash_file(physical) == pointer[1].decode(), 'LFS_INPUT_NOT_HYDRATED'
            value = str(physical)
        else:
            value = io.BytesIO(data)  # Git-normalized bytes, not Windows working-tree EOLs.
        operations.append(CommitOperationAdd(path_in_repo=path, path_or_fileobj=value))
    for path in sorted(remote.keys() - local.keys()):
        operations.append(CommitOperationDelete(path_in_repo=path))
    revision = api.create_commit(SPACE, repo_type='space', parent_commit=head,
        commit_message=f'Installed auth candidate; personal acceptance pending; source {commit}', operations=operations).oid if operations else head
    published = tree(api, SPACE, 'space')
    assert set(local) == set(published), 'SOURCE_PATH_SET_MISMATCH'
    mismatches = [p for p, blob in local.items() if published[p].blob_id != blob]
    assert not mismatches, 'SOURCE_BLOB_MISMATCH'
    print(json.dumps({'mode': 'source', 'git_commit': commit, 'revision': revision, 'files': len(local), 'tree_matches': True}))


if __name__ == '__main__':
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument('mode', choices=['artifact', 'source'])
    parser.add_argument('--version', default='v0.1.71')
    parser.add_argument('--space-parent')
    args = parser.parse_args()
    if os.geteuid() == 0:
        raise SystemExit('HOST_USER_REQUIRED_NOT_RUNTIME_ROOT')
    if not re.fullmatch(r'v[0-9]+\.[0-9]+\.[0-9]+', args.version):
        raise SystemExit('INVALID_CANDIDATE_VERSION')
    try:
        api = HfApi()  # Uses host-user cached credentials; never prints token.
        if args.mode == 'artifact':
            artifact(api, args.version)
        else:
            assert args.space_parent and re.fullmatch('[0-9a-f]{40}', args.space_parent), 'EXPLICIT_SPACE_PARENT_REQUIRED'
            source(api, args.space_parent)
    except Exception as error:
        print(json.dumps({'mode': args.mode, 'result': 'fail', 'exception_type': type(error).__name__,
                          'code': str(error) if isinstance(error, AssertionError) and re.fullmatch('[A-Z_]{1,64}', str(error)) else 'PUBLICATION_FAILED'}))
        raise SystemExit(1)
