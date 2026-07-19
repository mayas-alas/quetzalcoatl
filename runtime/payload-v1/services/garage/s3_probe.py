#!/usr/bin/python3
import datetime
import hashlib
import hmac
import http.client
import re
import secrets
import sys

HOST = "127.0.0.1:3900"
REGION = "garage"
SERVICE = "s3"
PATH = "/gnx-i1/i1-evidence.bin"


def hmac_sha256(key, value):
    return hmac.new(key, value, hashlib.sha256).digest()


def request(method, body, access_key, secret_key):
    now = datetime.datetime.now(datetime.timezone.utc)
    amz_date = now.strftime("%Y%m%dT%H%M%SZ")
    date_stamp = now.strftime("%Y%m%d")
    payload_hash = hashlib.sha256(body).hexdigest()
    canonical_headers = (
        f"host:{HOST}\n"
        f"x-amz-content-sha256:{payload_hash}\n"
        f"x-amz-date:{amz_date}\n"
    )
    signed_headers = "host;x-amz-content-sha256;x-amz-date"
    canonical_request = "\n".join(
        [
            method,
            PATH,
            "",
            canonical_headers,
            signed_headers,
            payload_hash,
        ]
    )
    scope = f"{date_stamp}/{REGION}/{SERVICE}/aws4_request"
    string_to_sign = "\n".join(
        [
            "AWS4-HMAC-SHA256",
            amz_date,
            scope,
            hashlib.sha256(canonical_request.encode("ascii")).hexdigest(),
        ]
    )
    date_key = hmac_sha256(("AWS4" + secret_key).encode("ascii"), date_stamp.encode("ascii"))
    region_key = hmac_sha256(date_key, REGION.encode("ascii"))
    service_key = hmac_sha256(region_key, SERVICE.encode("ascii"))
    signing_key = hmac_sha256(service_key, b"aws4_request")
    signature = hmac.new(signing_key, string_to_sign.encode("ascii"), hashlib.sha256).hexdigest()
    authorization = (
        f"AWS4-HMAC-SHA256 Credential={access_key}/{scope}, "
        f"SignedHeaders={signed_headers}, Signature={signature}"
    )
    headers = {
        "Authorization": authorization,
        "Host": HOST,
        "x-amz-content-sha256": payload_hash,
        "x-amz-date": amz_date,
    }
    connection = http.client.HTTPConnection("127.0.0.1", 3900, timeout=30)
    try:
        connection.request(method, PATH, body=body, headers=headers)
        response = connection.getresponse()
        response_body = response.read()
        return response.status, response_body
    finally:
        connection.close()


def main():
    access_key = sys.stdin.readline().strip()
    secret_key = sys.stdin.readline().strip()
    if not re.fullmatch(r"GK[0-9a-f]{24}", access_key):
        raise ValueError("invalid access key")
    if not re.fullmatch(r"[0-9a-f]{64}", secret_key):
        raise ValueError("invalid secret key")

    body = secrets.token_bytes(64)
    put_status, _ = request("PUT", body, access_key, secret_key)
    if put_status != 200:
        raise RuntimeError("PUT failed")
    get_status, received = request("GET", b"", access_key, secret_key)
    if get_status != 200 or not hmac.compare_digest(received, body):
        raise RuntimeError("GET failed")
    print(
        "S3_PUT_GET=ready;BYTES=64;BODY_SHA256="
        + hashlib.sha256(body).hexdigest()
    )


if __name__ == "__main__":
    try:
        main()
    except Exception:
        print("S3 functional probe failed", file=sys.stderr)
        raise SystemExit(1)
