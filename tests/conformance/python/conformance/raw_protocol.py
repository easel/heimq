"""A minimal, hand-rolled Kafka protocol client.

Two workloads need to send frames no client library will send: a duplicate
idempotent sequence number, and one that skips ahead. That requires building
Produce requests by hand.

The Rust original built these frames with `heimq_protocol` -- the server's own
protocol library -- so a bug in that encoder would be invisible: both sides of
the comparison shared it. Encoding independently here is the point.

Implements only what the workloads use: ApiVersions v0, InitProducerId v0,
Produce v3, and a v2 RecordBatch.
"""

import socket
import struct
from typing import Any

from crc32c import crc32c

CLIENT_ID = b"heimq-conformance-raw"
SOCKET_TIMEOUT = 15.0


def _put_nullable_string(buf: bytearray, s: str | None) -> None:
    if s is None:
        buf += struct.pack(">h", -1)
    else:
        raw = s.encode()
        buf += struct.pack(">h", len(raw)) + raw


def _put_string(buf: bytearray, s: str) -> None:
    raw = s.encode()
    buf += struct.pack(">h", len(raw)) + raw


def _varint(value: int) -> bytes:
    """Zig-zag encoded signed varint, as used by the v2 record format."""
    value = (value << 1) ^ (value >> 63) if value < 0 else value << 1
    out = bytearray()
    while True:
        chunk = value & 0x7F
        value >>= 7
        if value:
            out.append(chunk | 0x80)
        else:
            out.append(chunk)
            return bytes(out)


class Reader:
    def __init__(self, buf: bytes):
        self.buf = buf
        self.pos = 0

    def _take(self, n: int) -> bytes:
        b = self.buf[self.pos : self.pos + n]
        if len(b) != n:
            raise EOFError("short read")
        self.pos += n
        return b

    def int16(self) -> int:
        return struct.unpack(">h", self._take(2))[0]

    def int32(self) -> int:
        return struct.unpack(">i", self._take(4))[0]

    def int64(self) -> int:
        return struct.unpack(">q", self._take(8))[0]

    def string(self) -> str | None:
        n = self.int16()
        if n < 0:
            return None
        return self._take(n).decode()


def encode_record_batch(
    producer_id: int, producer_epoch: int, base_sequence: int
) -> bytes:
    """One record, v2 batch, no compression -- mirrors encode_idempotent_batch."""
    key = f"key-{base_sequence}".encode()
    value = f"value-{base_sequence}".encode()
    timestamp = base_sequence

    record = bytearray()
    record += struct.pack(">b", 0)  # attributes
    record += _varint(0)  # timestamp delta
    record += _varint(0)  # offset delta
    record += _varint(len(key)) + key
    record += _varint(len(value)) + value
    record += _varint(0)  # header count
    record_with_len = _varint(len(record)) + bytes(record)

    # Everything the CRC covers: attributes through the record set.
    body = bytearray()
    body += struct.pack(">h", 0)  # attributes
    body += struct.pack(">i", 0)  # last offset delta
    body += struct.pack(">q", timestamp)  # base timestamp
    body += struct.pack(">q", timestamp)  # max timestamp
    body += struct.pack(">q", producer_id)
    body += struct.pack(">h", producer_epoch)
    body += struct.pack(">i", base_sequence)
    body += struct.pack(">i", 1)  # record count
    body += record_with_len

    crc = crc32c(bytes(body))

    after_batch_len = bytearray()
    after_batch_len += struct.pack(">i", 0)  # partition leader epoch
    after_batch_len += struct.pack(">b", 2)  # magic
    after_batch_len += struct.pack(">I", crc)
    after_batch_len += body

    batch = bytearray()
    batch += struct.pack(">q", 0)  # base offset
    batch += struct.pack(">i", len(after_batch_len))
    batch += after_batch_len
    return bytes(batch)


class RawKafkaClient:
    def __init__(self, bootstrap: str):
        host, port = bootstrap.rsplit(":", 1)
        self.sock = socket.create_connection((host, int(port)), timeout=SOCKET_TIMEOUT)
        self.sock.settimeout(SOCKET_TIMEOUT)
        self.correlation_id = 1

    def close(self) -> None:
        self.sock.close()

    def __enter__(self) -> "RawKafkaClient":
        return self

    def __exit__(self, *exc: Any) -> None:
        self.close()

    def _send(self, api_key: int, api_version: int, body: bytes) -> Reader:
        correlation_id = self.correlation_id
        self.correlation_id += 1

        header = bytearray()
        header += struct.pack(">hhi", api_key, api_version, correlation_id)
        header += struct.pack(">h", len(CLIENT_ID)) + CLIENT_ID

        frame = bytes(header) + body
        self.sock.sendall(struct.pack(">i", len(frame)) + frame)

        (length,) = struct.unpack(">i", self._read_exact(4))
        if length < 4:
            raise RuntimeError(f"invalid response frame length {length}")
        r = Reader(self._read_exact(length))
        got = r.int32()
        if got != correlation_id:
            raise RuntimeError(
                f"correlation id mismatch: request {correlation_id}, response {got}"
            )
        return r

    def _read_exact(self, n: int) -> bytes:
        chunks = []
        remaining = n
        while remaining:
            chunk = self.sock.recv(remaining)
            if not chunk:
                raise EOFError("connection closed")
            chunks.append(chunk)
            remaining -= len(chunk)
        return b"".join(chunks)

    def api_versions(self) -> int:
        r = self._send(18, 0, b"")
        return r.int16()  # error_code

    def init_producer_id(self) -> tuple[int, int]:
        body = bytearray()
        _put_nullable_string(body, None)  # transactional_id
        body += struct.pack(">i", 60_000)  # transaction_timeout_ms

        r = self._send(22, 0, bytes(body))
        r.int32()  # throttle_time_ms
        error_code = r.int16()
        if error_code != 0:
            raise RuntimeError(f"InitProducerId returned error {error_code}")
        producer_id = r.int64()
        producer_epoch = r.int16()
        return producer_id, producer_epoch

    def produce_sequence(
        self, topic: str, producer_id: int, producer_epoch: int, base_sequence: int
    ) -> int:
        records = encode_record_batch(producer_id, producer_epoch, base_sequence)

        body = bytearray()
        _put_nullable_string(body, None)  # transactional_id
        body += struct.pack(">h", 1)  # acks
        body += struct.pack(">i", 15_000)  # timeout_ms
        body += struct.pack(">i", 1)  # topic_data count
        _put_string(body, topic)
        body += struct.pack(">i", 1)  # partition_data count
        body += struct.pack(">i", 0)  # partition index
        body += struct.pack(">i", len(records)) + records

        r = self._send(0, 3, bytes(body))
        topic_count = r.int32()
        if topic_count < 1:
            raise RuntimeError("Produce response missing topic response")
        r.string()  # topic name
        partition_count = r.int32()
        if partition_count < 1:
            raise RuntimeError("Produce response missing partition response")
        r.int32()  # partition index
        return r.int16()  # error_code
