"""Reproducible test-only CA and server certificate.

The recording snapshot (rust/app-cases/files/test-env/certs/ca.crt) must trust
the live HTTPS server on every fresh checkout. Fixed EC keys from published
seeds guarantee the same key pair and trust anchor; ECDSA signatures are
randomized, so certificate bytes differ between runs by design.

These keys are test-only, never trusted by the system store. They are written
to test-env/certs/ (gitignored) and must never be committed.
"""

import ipaddress

import hashlib
from datetime import datetime, timezone
from pathlib import Path

from cryptography import x509
from cryptography.hazmat.primitives import hashes, serialization
from cryptography.hazmat.primitives.asymmetric import ec
from cryptography.x509.oid import ExtendedKeyUsageOID, NameOID

CERTS = Path(__file__).resolve().parent / "certs"
NOT_BEFORE = datetime(2026, 1, 1, tzinfo=timezone.utc)
NOT_AFTER = datetime(2036, 1, 1, tzinfo=timezone.utc)
CURVE = ec.SECP256R1()

# NIST P-256 group order (public domain constant).
P256_ORDER = 0xFFFFFFFF00000000FFFFFFFFFFFFFFFFBCE6FAADA7179E84F3B9CAC2FC632551


def fixed_key(seed: str) -> ec.EllipticCurvePrivateKey:
    digest = hashlib.sha256(seed.encode("utf-8")).digest()
    scalar = int.from_bytes(digest, "big") % (P256_ORDER - 1) + 1
    return ec.derive_private_key(scalar, CURVE)


def name(common: str) -> x509.Name:
    return x509.Name([x509.NameAttribute(NameOID.COMMON_NAME, common)])


def write_pem(path: Path, data: bytes, mode=0o644):
    path.write_bytes(data)
    path.chmod(mode)


def main():
    CERTS.mkdir(parents=True, exist_ok=True)
    ca_key = fixed_key("aria2-rs test CA seed v1")
    server_key = fixed_key("aria2-rs test server seed v1")

    ca_cert = (
        x509.CertificateBuilder()
        .subject_name(name("aria2-rs test CA"))
        .issuer_name(name("aria2-rs test CA"))
        .public_key(ca_key.public_key())
        .serial_number(1)
        .not_valid_before(NOT_BEFORE)
        .not_valid_after(NOT_AFTER)
        .add_extension(x509.BasicConstraints(ca=True, path_length=None), critical=True)
        .sign(ca_key, hashes.SHA256())
    )
    server_cert = (
        x509.CertificateBuilder()
        .subject_name(name("localhost"))
        .issuer_name(ca_cert.subject)
        .public_key(server_key.public_key())
        .serial_number(2)
        .not_valid_before(NOT_BEFORE)
        .not_valid_after(NOT_AFTER)
        .add_extension(
            x509.SubjectAlternativeName(
                [x509.DNSName("localhost"), x509.IPAddress(ipaddress.ip_address("127.0.0.1"))]
            ),
            critical=False,
        )
        .add_extension(
            x509.ExtendedKeyUsage([ExtendedKeyUsageOID.SERVER_AUTH]), critical=False
        )
        .add_extension(x509.BasicConstraints(ca=False, path_length=None), critical=True)
        .sign(ca_key, hashes.SHA256())
    )

    key_pem = lambda key: key.private_bytes(
        serialization.Encoding.PEM,
        serialization.PrivateFormat.PKCS8,
        serialization.NoEncryption(),
    )
    write_pem(CERTS / "ca.key", key_pem(ca_key), 0o600)
    write_pem(CERTS / "server.key", key_pem(server_key), 0o600)
    write_pem(CERTS / "ca.crt", ca_cert.public_bytes(serialization.Encoding.PEM))
    write_pem(CERTS / "server.crt", server_cert.public_bytes(serialization.Encoding.PEM))
    print("deterministic test certificates written to test-env/certs/")


if __name__ == "__main__":
    main()
