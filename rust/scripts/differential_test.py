#!/usr/bin/env python3
import argparse
import base64
import datetime as dt
import difflib
import hashlib
import hmac
import http.client
import json
import os
import pathlib
import re
import shutil
import socket
import subprocess
import sys
import tempfile
import threading
import time
import urllib.parse
import uuid
import xml.etree.ElementTree as ET
from dataclasses import dataclass, field
from http.server import BaseHTTPRequestHandler, ThreadingHTTPServer
from typing import Callable, Iterable, Optional

ACCOUNT_NAME = "devstoreaccount1"
ACCOUNT_KEY = (
    "Eby8vdM02xNOcqFlqUwJPLlmEtlCDXJ1OUzFT50uSRZ6IFsuFq2UVErCz4I6tq/K1SZFPTOtr/KBHBeksoGMGw=="
)
API_VERSION = "2021-12-02"
HOST_HEADER = "azurite.test"
COPY_SOURCE_PORT = 12080
COPY_SOURCE_PATH = "/copy-source.bin"
COPY_SOURCE_BYTES = b"copied from differential harness\n"
LEASE_ID = "11111111-1111-1111-1111-111111111111"
BLOB_PORTS = {"ts": 10000, "rust": 11000}
QUEUE_PORTS = {"ts": 10001, "rust": 11001}
TABLE_PORTS = {"ts": 10002, "rust": 11002}

TRANSPORT_IGNORED_HEADER_NAMES = {
    "connection",
    "keep-alive",
    "server",
    "transfer-encoding",
}

DYNAMIC_HEADER_PLACEHOLDERS = {
    "date": "DYNAMIC_DATE",
    "etag": "DYNAMIC_ETAG",
    "x-ms-copy-id": "DYNAMIC_COPY_ID",
    "x-ms-request-id": "DYNAMIC_REQUEST_ID",
    "x-ms-snapshot": "DYNAMIC_SNAPSHOT",
    "x-ms-version-id": "DYNAMIC_VERSION_ID",
}

DYNAMIC_FIELD_PLACEHOLDERS = {
    "clientrequestid": "DYNAMIC_REQUEST_ID",
    "date": "DYNAMIC_DATE",
    "etag": "DYNAMIC_ETAG",
    "expirationtime": "DYNAMIC_TIMESTAMP",
    "insertiontime": "DYNAMIC_TIMESTAMP",
    "lastmodified": "DYNAMIC_TIMESTAMP",
    "lastmodifiedtime": "DYNAMIC_TIMESTAMP",
    "messageid": "DYNAMIC_MESSAGE_ID",
    "nextvisibletime": "DYNAMIC_TIMESTAMP",
    "odata.etag": "DYNAMIC_ETAG",
    "popreceipt": "DYNAMIC_POP_RECEIPT",
    "requestid": "DYNAMIC_REQUEST_ID",
    "serviceendpoint": "DYNAMIC_ENDPOINT",
    "time": "DYNAMIC_TIMESTAMP",
    "timestamp": "DYNAMIC_TIMESTAMP",
    "timenextvisible": "DYNAMIC_TIMESTAMP",
}

REQUEST_ID_MESSAGE_RE = re.compile(r"RequestId:\s*[^<\r\n]+")
TIME_MESSAGE_RE = re.compile(r"Time:\s*[^<\r\n]+")
HEX_ETAG_RE = re.compile(r"0x[0-9A-Fa-f]+")
QUOTED_HEX_ETAG_RE = re.compile(r'"0x[0-9A-Fa-f]+"')
WEAK_ETAG_RE = re.compile(r'W/"[^"]+"')

REPO_ROOT = pathlib.Path(__file__).resolve().parents[2]
RUST_ROOT = REPO_ROOT / "rust"
SCRIPT_DIR = pathlib.Path(__file__).resolve().parent


@dataclass
class RequestSpec:
    method: str
    service: str
    path: str
    query: list[tuple[str, str]] = field(default_factory=list)
    headers: list[tuple[str, str]] = field(default_factory=list)
    body: bytes = b""
    expect: str = "auto"
    ignore_body_fields: set[str] = field(default_factory=set)


@dataclass
class HttpResponseData:
    status: int
    reason: str
    headers: list[tuple[str, str]]
    body: bytes


@dataclass
class ComparisonOutcome:
    ok: bool
    summary: str
    details: list[str] = field(default_factory=list)


@dataclass
class ScenarioResult:
    name: str
    ok: bool
    summary: str
    details: list[str] = field(default_factory=list)


class StaticCopySourceHandler(BaseHTTPRequestHandler):
    def do_GET(self) -> None:  # noqa: N802
        if self.path != COPY_SOURCE_PATH:
            self.send_error(404)
            return
        self.send_response(200)
        self.send_header("Content-Length", str(len(COPY_SOURCE_BYTES)))
        self.send_header("Content-Type", "application/octet-stream")
        self.end_headers()
        self.wfile.write(COPY_SOURCE_BYTES)

    def log_message(self, format: str, *args: object) -> None:  # noqa: A003
        return


class CopySourceServer:
    def __init__(self) -> None:
        self.server = ThreadingHTTPServer(("127.0.0.1", COPY_SOURCE_PORT), StaticCopySourceHandler)
        self.thread = threading.Thread(target=self.server.serve_forever, daemon=True)

    def start(self) -> None:
        self.thread.start()

    def stop(self) -> None:
        self.server.shutdown()
        self.server.server_close()
        self.thread.join(timeout=5)


class ProcessGroup:
    def __init__(self, artifacts_dir: pathlib.Path) -> None:
        self.artifacts_dir = artifacts_dir
        self.processes: list[tuple[str, subprocess.Popen[bytes], pathlib.Path]] = []

    def start(self, name: str, command: list[str], cwd: pathlib.Path) -> None:
        log_path = self.artifacts_dir / f"{name}.log"
        with log_path.open("wb") as log_file:
            process = subprocess.Popen(
                command,
                cwd=str(cwd),
                stdout=log_file,
                stderr=subprocess.STDOUT,
                preexec_fn=os.setsid,
            )
        self.processes.append((name, process, log_path))

    def stop(self) -> None:
        for name, process, _log_path in reversed(self.processes):
            if process.poll() is None:
                try:
                    os.killpg(process.pid, 15)
                    process.wait(timeout=10)
                except subprocess.TimeoutExpired:
                    os.killpg(process.pid, 9)
                    process.wait(timeout=5)
                except ProcessLookupError:
                    pass

    def ensure_alive(self) -> None:
        dead = []
        for name, process, log_path in self.processes:
            code = process.poll()
            if code is not None:
                dead.append(f"{name} exited with code {code} (log: {log_path})")
        if dead:
            raise RuntimeError("; ".join(dead))

    def log_paths(self) -> list[str]:
        return [str(log_path) for _name, _process, log_path in self.processes]


class DifferentialHarness:
    def __init__(self, keep_artifacts: bool, artifacts_dir: Optional[str]) -> None:
        self.keep_artifacts = keep_artifacts
        self._tempdir: Optional[tempfile.TemporaryDirectory[str]] = None
        if artifacts_dir:
            self.artifacts_dir = pathlib.Path(artifacts_dir).resolve()
            self.artifacts_dir.mkdir(parents=True, exist_ok=True)
        else:
            self._tempdir = tempfile.TemporaryDirectory(prefix="azurite-diff-")
            self.artifacts_dir = pathlib.Path(self._tempdir.name)
        self.ts_state_dir = self.artifacts_dir / "ts-state"
        self.rust_state_dir = self.artifacts_dir / "rust-state"
        self.ts_state_dir.mkdir(parents=True, exist_ok=True)
        self.rust_state_dir.mkdir(parents=True, exist_ok=True)
        self.processes = ProcessGroup(self.artifacts_dir)
        self.copy_source = CopySourceServer()
        suffix = uuid.uuid4().hex[:10]
        self.container_name = f"diffcontainer{suffix}"
        self.queue_name = f"diffqueue{suffix}"
        self.table_name = f"DiffTable{suffix}"
        self.block_blob_name = "block-blob.txt"
        self.page_blob_name = "page-blob.bin"
        self.copy_blob_name = "copied-blob.bin"
        self.ts_base_urls = {
            "blob": f"http://127.0.0.1:{BLOB_PORTS['ts']}",
            "queue": f"http://127.0.0.1:{QUEUE_PORTS['ts']}",
            "table": f"http://127.0.0.1:{TABLE_PORTS['ts']}",
        }
        self.rust_base_urls = {
            "blob": f"http://127.0.0.1:{BLOB_PORTS['rust']}",
            "queue": f"http://127.0.0.1:{QUEUE_PORTS['rust']}",
            "table": f"http://127.0.0.1:{TABLE_PORTS['rust']}",
        }
        self.request_counter = 0
        self.lease_blob_name = "lease-blob.txt"

    def cleanup(self) -> None:
        self.processes.stop()
        try:
            self.copy_source.stop()
        except Exception:
            pass
        if self._tempdir and not self.keep_artifacts:
            self._tempdir.cleanup()

    def ensure_rust_binaries(self) -> dict[str, pathlib.Path]:
        binaries = {
            "blob": self._find_binary("azurite-blob"),
            "queue": self._find_binary("azurite-queue"),
            "table": self._find_binary("azurite-table"),
        }
        if all(path is not None for path in binaries.values()):
            return {name: path for name, path in binaries.items() if path is not None}

        subprocess.run(
            [
                "cargo",
                "build",
                "--release",
                "--bin",
                "azurite-blob",
                "--bin",
                "azurite-queue",
                "--bin",
                "azurite-table",
            ],
            cwd=str(RUST_ROOT),
            check=True,
        )
        rebuilt = {
            "blob": self._find_binary("azurite-blob"),
            "queue": self._find_binary("azurite-queue"),
            "table": self._find_binary("azurite-table"),
        }
        missing = [name for name, path in rebuilt.items() if path is None]
        if missing:
            raise RuntimeError(f"Missing Rust binaries after build: {', '.join(missing)}")
        return {name: path for name, path in rebuilt.items() if path is not None}

    def _find_binary(self, name: str) -> Optional[pathlib.Path]:
        direct = RUST_ROOT / "target" / "release" / name
        if direct.exists() and os.access(direct, os.X_OK):
            return direct
        for candidate in sorted((RUST_ROOT / "target").glob(f"**/release/{name}")):
            if candidate.exists() and os.access(candidate, os.X_OK):
                return candidate
        return None

    def start_servers(self) -> None:
        rust_bins = self.ensure_rust_binaries()
        self.copy_source.start()
        self._wait_for_copy_source()

        self.processes.start(
            "ts-blob",
            [
                "node",
                "-r",
                "ts-node/register",
                "src/blob/main.ts",
                "--blobHost",
                "127.0.0.1",
                "--blobPort",
                str(BLOB_PORTS["ts"]),
                "--location",
                str(self.ts_state_dir / "blob"),
                "--silent",
                "--disableTelemetry",
                "--disableProductStyleUrl",
            ],
            REPO_ROOT,
        )
        self.processes.start(
            "ts-queue",
            [
                "node",
                "-r",
                "ts-node/register",
                "src/queue/main.ts",
                "--queueHost",
                "127.0.0.1",
                "--queuePort",
                str(QUEUE_PORTS["ts"]),
                "--location",
                str(self.ts_state_dir / "queue"),
                "--silent",
                "--disableTelemetry",
                "--disableProductStyleUrl",
            ],
            REPO_ROOT,
        )
        self.processes.start(
            "ts-table",
            [
                "node",
                "-r",
                "ts-node/register",
                "src/table/main.ts",
                "--tableHost",
                "127.0.0.1",
                "--tablePort",
                str(TABLE_PORTS["ts"]),
                "--location",
                str(self.ts_state_dir / "table"),
                "--silent",
                "--disableTelemetry",
                "--disableProductStyleUrl",
            ],
            REPO_ROOT,
        )

        self.processes.start(
            "rust-blob",
            [
                str(rust_bins["blob"]),
                "--blobHost",
                "127.0.0.1",
                "--blobPort",
                str(BLOB_PORTS["rust"]),
                "--location",
                str(self.rust_state_dir / "blob"),
                "--silent",
                "--disableTelemetry",
                "--disableProductStyleUrl",
            ],
            REPO_ROOT,
        )
        self.processes.start(
            "rust-queue",
            [
                str(rust_bins["queue"]),
                "--queueHost",
                "127.0.0.1",
                "--queuePort",
                str(QUEUE_PORTS["rust"]),
                "--location",
                str(self.rust_state_dir / "queue"),
                "--silent",
                "--disableTelemetry",
                "--disableProductStyleUrl",
            ],
            REPO_ROOT,
        )
        self.processes.start(
            "rust-table",
            [
                str(rust_bins["table"]),
                "--tableHost",
                "127.0.0.1",
                "--tablePort",
                str(TABLE_PORTS["rust"]),
                "--location",
                str(self.rust_state_dir / "table"),
                "--silent",
                "--disableTelemetry",
                "--disableProductStyleUrl",
            ],
            REPO_ROOT,
        )

        for name, port in [
            ("ts-blob", BLOB_PORTS["ts"]),
            ("ts-queue", QUEUE_PORTS["ts"]),
            ("ts-table", TABLE_PORTS["ts"]),
            ("rust-blob", BLOB_PORTS["rust"]),
            ("rust-queue", QUEUE_PORTS["rust"]),
            ("rust-table", TABLE_PORTS["rust"]),
        ]:
            self._wait_for_port(name, port)

    def _wait_for_port(self, name: str, port: int, timeout: float = 30.0) -> None:
        deadline = time.time() + timeout
        while time.time() < deadline:
            self.processes.ensure_alive()
            with socket.socket(socket.AF_INET, socket.SOCK_STREAM) as sock:
                sock.settimeout(1)
                if sock.connect_ex(("127.0.0.1", port)) == 0:
                    return
            time.sleep(0.25)
        raise RuntimeError(f"Timed out waiting for {name} on port {port}")

    def _wait_for_copy_source(self, timeout: float = 5.0) -> None:
        deadline = time.time() + timeout
        while time.time() < deadline:
            with socket.socket(socket.AF_INET, socket.SOCK_STREAM) as sock:
                sock.settimeout(1)
                if sock.connect_ex(("127.0.0.1", COPY_SOURCE_PORT)) == 0:
                    return
            time.sleep(0.05)
        raise RuntimeError(f"Timed out waiting for copy source on port {COPY_SOURCE_PORT}")

    def run(self) -> list[ScenarioResult]:
        scenarios: list[tuple[str, Callable[[], ScenarioResult]]] = [
            ("Create container", self.scenario_create_container),
            ("List containers", self.scenario_list_containers),
            ("Upload block blob", self.scenario_upload_block_blob),
            ("Download block blob", self.scenario_download_block_blob),
            ("Get blob properties", self.scenario_get_blob_properties),
            ("Create page blob + page ranges", self.scenario_page_blob_ranges),
            ("Lease operations", self.scenario_lease_operations),
            ("Create snapshot", self.scenario_create_snapshot),
            ("Copy blob", self.scenario_copy_blob),
            ("Set/get blob metadata", self.scenario_blob_metadata),
            ("Append blob operations", self.scenario_append_blob_operations),
            ("Blob snapshots with metadata", self.scenario_snapshot_metadata),
            ("List blobs", self.scenario_list_blobs),
            ("Delete blob", self.scenario_delete_blob),
            ("Create queue", self.scenario_create_queue),
            ("Put message", self.scenario_put_message),
            ("Get messages", self.scenario_get_messages),
            ("Delete message", self.scenario_delete_message),
            ("Delete queue", self.scenario_delete_queue),
            ("Create table", self.scenario_create_table),
            ("Insert entity", self.scenario_insert_entity),
            ("Get entity", self.scenario_get_entity),
            ("Query entities", self.scenario_query_entities),
            ("Delete entity", self.scenario_delete_entity),
        ]
        results = []
        for name, fn in scenarios:
            try:
                results.append(fn())
            except Exception as exc:
                results.append(
                    ScenarioResult(
                        name=name,
                        ok=False,
                        summary=str(exc),
                        details=[f"Harness error while running scenario: {exc}"],
                    )
                )
        return results

    def perform_variant_requests(
        self, ts_spec: RequestSpec, rust_spec: RequestSpec
    ) -> tuple[HttpResponseData, HttpResponseData]:
        self.request_counter += 1
        request_id = f"diff-{self.request_counter:04d}"
        date_string = format_rfc1123(dt.datetime.now(dt.timezone.utc))
        ts_response = self._perform_request("ts", ts_spec, date_string, request_id)
        rust_response = self._perform_request("rust", rust_spec, date_string, request_id)
        return ts_response, rust_response

    def execute_and_compare(self, spec: RequestSpec) -> ComparisonOutcome:
        ts_response, rust_response = self.perform_variant_requests(spec, spec)
        return compare_responses(ts_response, rust_response, spec)

    def execute_variant_compare(
        self,
        ts_spec: RequestSpec,
        rust_spec: RequestSpec,
        comparison_spec: Optional[RequestSpec] = None,
    ) -> tuple[ComparisonOutcome, HttpResponseData, HttpResponseData]:
        ts_response, rust_response = self.perform_variant_requests(ts_spec, rust_spec)
        outcome = compare_responses(ts_response, rust_response, comparison_spec or ts_spec)
        return outcome, ts_response, rust_response

    def _perform_request(
        self, variant: str, spec: RequestSpec, date_string: str, request_id: str
    ) -> HttpResponseData:
        port = {
            "blob": BLOB_PORTS[variant],
            "queue": QUEUE_PORTS[variant],
            "table": TABLE_PORTS[variant],
        }[spec.service]
        query_string = urllib.parse.urlencode(spec.query, doseq=True, safe="$'(),")
        target = spec.path if not query_string else f"{spec.path}?{query_string}"
        headers = [("Host", HOST_HEADER), ("Accept-Encoding", "identity")]
        headers.extend(spec.headers)
        lower_names = {name.lower() for name, _value in headers}
        if spec.service == "table":
            if "date" not in lower_names:
                headers.append(("Date", date_string))
            if "x-ms-version" not in lower_names:
                headers.append(("x-ms-version", API_VERSION))
            if "data-service-version" not in lower_names:
                headers.append(("DataServiceVersion", "3.0;NetFx"))
            if "maxdataserviceversion" not in lower_names and "max-data-service-version" not in lower_names:
                headers.append(("MaxDataServiceVersion", "3.0;NetFx"))
            if "accept" not in lower_names:
                headers.append(("Accept", "application/json;odata=nometadata"))
        else:
            if "x-ms-date" not in lower_names:
                headers.append(("x-ms-date", date_string))
            if "x-ms-version" not in lower_names:
                headers.append(("x-ms-version", API_VERSION))
        if "x-ms-client-request-id" not in lower_names:
            headers.append(("x-ms-client-request-id", request_id))
        if "content-length" not in lower_names:
            headers.append(("Content-Length", str(len(spec.body))))
        if spec.body and "content-type" not in lower_names:
            headers.append(("Content-Type", "application/octet-stream"))

        authorization = build_authorization(spec, headers, spec.body)
        headers.append(("Authorization", authorization))

        connection = http.client.HTTPConnection("127.0.0.1", port, timeout=30)
        connection.putrequest(spec.method, target, skip_host=True, skip_accept_encoding=True)
        for name, value in headers:
            connection.putheader(name, value)
        connection.endheaders(spec.body)
        response = connection.getresponse()
        body = response.read()
        headers_list = response.getheaders()
        reason = response.reason or ""
        connection.close()
        return HttpResponseData(status=response.status, reason=reason, headers=headers_list, body=body)

    def scenario_create_container(self) -> ScenarioResult:
        spec = RequestSpec(
            method="PUT",
            service="blob",
            path=f"/{ACCOUNT_NAME}/{self.container_name}",
            query=[("restype", "container")],
        )
        return outcome_to_scenario("Create container", self.execute_and_compare(spec))

    def scenario_list_containers(self) -> ScenarioResult:
        spec = RequestSpec(
            method="GET",
            service="blob",
            path=f"/{ACCOUNT_NAME}",
            query=[("comp", "list")],
            headers=[("Accept", "application/xml")],
            ignore_body_fields={"serviceendpoint"},
        )
        return outcome_to_scenario("List containers", self.execute_and_compare(spec))

    def scenario_upload_block_blob(self) -> ScenarioResult:
        body = b"hello from differential harness"
        spec = RequestSpec(
            method="PUT",
            service="blob",
            path=f"/{ACCOUNT_NAME}/{self.container_name}/{self.block_blob_name}",
            body=body,
            headers=[
                ("x-ms-blob-type", "BlockBlob"),
                ("Content-Type", "text/plain"),
            ],
        )
        return outcome_to_scenario("Upload block blob", self.execute_and_compare(spec))

    def scenario_download_block_blob(self) -> ScenarioResult:
        spec = RequestSpec(
            method="GET",
            service="blob",
            path=f"/{ACCOUNT_NAME}/{self.container_name}/{self.block_blob_name}",
            expect="binary",
        )
        return outcome_to_scenario("Download block blob", self.execute_and_compare(spec))

    def scenario_get_blob_properties(self) -> ScenarioResult:
        spec = RequestSpec(
            method="HEAD",
            service="blob",
            path=f"/{ACCOUNT_NAME}/{self.container_name}/{self.block_blob_name}",
        )
        return outcome_to_scenario("Get blob properties", self.execute_and_compare(spec))

    def scenario_page_blob_ranges(self) -> ScenarioResult:
        create_spec = RequestSpec(
            method="PUT",
            service="blob",
            path=f"/{ACCOUNT_NAME}/{self.container_name}/{self.page_blob_name}",
            headers=[
                ("x-ms-blob-type", "PageBlob"),
                ("x-ms-blob-content-length", "512"),
            ],
        )
        create_outcome = self.execute_and_compare(create_spec)
        if not create_outcome.ok:
            return outcome_to_scenario("Create page blob + page ranges", create_outcome)
        page_body = b"A" * 512
        upload_pages = RequestSpec(
            method="PUT",
            service="blob",
            path=f"/{ACCOUNT_NAME}/{self.container_name}/{self.page_blob_name}",
            query=[("comp", "page")],
            body=page_body,
            headers=[
                ("x-ms-page-write", "update"),
                ("x-ms-range", "bytes=0-511"),
                ("Content-Type", "application/octet-stream"),
            ],
        )
        upload_outcome = self.execute_and_compare(upload_pages)
        if not upload_outcome.ok:
            return outcome_to_scenario("Create page blob + page ranges", upload_outcome)
        page_ranges = RequestSpec(
            method="GET",
            service="blob",
            path=f"/{ACCOUNT_NAME}/{self.container_name}/{self.page_blob_name}",
            query=[("comp", "pagelist")],
            headers=[("Accept", "application/xml")],
        )
        return outcome_to_scenario("Create page blob + page ranges", self.execute_and_compare(page_ranges))

    def scenario_lease_operations(self) -> ScenarioResult:
        seed_blob = RequestSpec(
            method="PUT",
            service="blob",
            path=f"/{ACCOUNT_NAME}/{self.container_name}/{self.lease_blob_name}",
            body=b"lease-target",
            headers=[
                ("x-ms-blob-type", "BlockBlob"),
                ("Content-Type", "text/plain"),
            ],
        )
        self.execute_and_compare(seed_blob)
        acquire = RequestSpec(
            method="PUT",
            service="blob",
            path=f"/{ACCOUNT_NAME}/{self.container_name}/{self.lease_blob_name}",
            query=[("comp", "lease")],
            headers=[
                ("x-ms-lease-action", "acquire"),
                ("x-ms-lease-duration", "15"),
                ("x-ms-proposed-lease-id", LEASE_ID),
            ],
        )
        acquire_outcome = self.execute_and_compare(acquire)
        if not acquire_outcome.ok:
            return outcome_to_scenario("Lease operations", acquire_outcome)
        renew = RequestSpec(
            method="PUT",
            service="blob",
            path=f"/{ACCOUNT_NAME}/{self.container_name}/{self.lease_blob_name}",
            query=[("comp", "lease")],
            headers=[
                ("x-ms-lease-action", "renew"),
                ("x-ms-lease-id", LEASE_ID),
            ],
        )
        renew_outcome = self.execute_and_compare(renew)
        if not renew_outcome.ok:
            return outcome_to_scenario("Lease operations", renew_outcome)
        release = RequestSpec(
            method="PUT",
            service="blob",
            path=f"/{ACCOUNT_NAME}/{self.container_name}/{self.lease_blob_name}",
            query=[("comp", "lease")],
            headers=[
                ("x-ms-lease-action", "release"),
                ("x-ms-lease-id", LEASE_ID),
            ],
        )
        return outcome_to_scenario("Lease operations", self.execute_and_compare(release))

    def scenario_create_snapshot(self) -> ScenarioResult:
        spec = RequestSpec(
            method="PUT",
            service="blob",
            path=f"/{ACCOUNT_NAME}/{self.container_name}/{self.block_blob_name}",
            query=[("comp", "snapshot")],
        )
        return outcome_to_scenario("Create snapshot", self.execute_and_compare(spec))

    def scenario_copy_blob(self) -> ScenarioResult:
        spec = RequestSpec(
            method="PUT",
            service="blob",
            path=f"/{ACCOUNT_NAME}/{self.container_name}/{self.copy_blob_name}",
            headers=[
                ("x-ms-blob-type", "BlockBlob"),
                (
                    "x-ms-copy-source",
                    f"http://127.0.0.1:{COPY_SOURCE_PORT}{COPY_SOURCE_PATH}",
                ),
            ],
        )
        return outcome_to_scenario("Copy blob", self.execute_and_compare(spec))

    def scenario_blob_metadata(self) -> ScenarioResult:
        set_spec = RequestSpec(
            method="PUT",
            service="blob",
            path=f"/{ACCOUNT_NAME}/{self.container_name}/{self.block_blob_name}",
            query=[("comp", "metadata")],
            headers=[
                ("x-ms-meta-purpose", "diff"),
                ("x-ms-meta-owner", "boromir"),
            ],
        )
        set_outcome = self.execute_and_compare(set_spec)
        if not set_outcome.ok:
            return outcome_to_scenario("Set/get blob metadata", set_outcome)
        get_spec = RequestSpec(
            method="HEAD",
            service="blob",
            path=f"/{ACCOUNT_NAME}/{self.container_name}/{self.block_blob_name}",
        )
        return outcome_to_scenario("Set/get blob metadata", self.execute_and_compare(get_spec))

    def scenario_append_blob_operations(self) -> ScenarioResult:
        blob_name = "append-blob.txt"
        create_spec = RequestSpec(
            method="PUT",
            service="blob",
            path=f"/{ACCOUNT_NAME}/{self.container_name}/{blob_name}",
            headers=[("x-ms-blob-type", "AppendBlob")],
        )
        create_outcome = self.execute_and_compare(create_spec)
        if not create_outcome.ok:
            return outcome_to_scenario("Append blob operations", create_outcome)
        append_spec = RequestSpec(
            method="PUT",
            service="blob",
            path=f"/{ACCOUNT_NAME}/{self.container_name}/{blob_name}",
            query=[("comp", "appendblock")],
            body=b"append-line-1\nappend-line-2\n",
            headers=[("Content-Type", "text/plain")],
        )
        append_outcome = self.execute_and_compare(append_spec)
        if not append_outcome.ok:
            return outcome_to_scenario("Append blob operations", append_outcome)
        download_spec = RequestSpec(
            method="GET",
            service="blob",
            path=f"/{ACCOUNT_NAME}/{self.container_name}/{blob_name}",
            expect="binary",
        )
        return outcome_to_scenario("Append blob operations", self.execute_and_compare(download_spec))

    def scenario_snapshot_metadata(self) -> ScenarioResult:
        blob_name = "snapshot-metadata.txt"
        seed_spec = RequestSpec(
            method="PUT",
            service="blob",
            path=f"/{ACCOUNT_NAME}/{self.container_name}/{blob_name}",
            body=b"snapshot source",
            headers=[
                ("x-ms-blob-type", "BlockBlob"),
                ("Content-Type", "text/plain"),
            ],
        )
        seed_outcome = self.execute_and_compare(seed_spec)
        if not seed_outcome.ok:
            return outcome_to_scenario("Blob snapshots with metadata", seed_outcome)
        snapshot_spec = RequestSpec(
            method="PUT",
            service="blob",
            path=f"/{ACCOUNT_NAME}/{self.container_name}/{blob_name}",
            query=[("comp", "snapshot")],
            headers=[
                ("x-ms-meta-purpose", "snapshot"),
                ("x-ms-meta-owner", "boromir"),
            ],
        )
        snapshot_outcome, ts_response, rust_response = self.execute_variant_compare(snapshot_spec, snapshot_spec)
        if not snapshot_outcome.ok:
            return outcome_to_scenario("Blob snapshots with metadata", snapshot_outcome)
        ts_snapshot = first_header_value(ordered_header_map(ts_response.headers), "x-ms-snapshot")
        rust_snapshot = first_header_value(ordered_header_map(rust_response.headers), "x-ms-snapshot")
        if not ts_snapshot or not rust_snapshot:
            return ScenarioResult(
                name="Blob snapshots with metadata",
                ok=False,
                summary="Snapshot identifier missing from create response",
                details=[
                    f"TS x-ms-snapshot={ts_snapshot or '<missing>'}",
                    f"Rust x-ms-snapshot={rust_snapshot or '<missing>'}",
                ],
            )
        comparison_spec = RequestSpec(
            method="HEAD",
            service="blob",
            path=f"/{ACCOUNT_NAME}/{self.container_name}/{blob_name}",
        )
        head_outcome, _ts_head, _rust_head = self.execute_variant_compare(
            RequestSpec(
                method="HEAD",
                service="blob",
                path=f"/{ACCOUNT_NAME}/{self.container_name}/{blob_name}",
                query=[("snapshot", ts_snapshot)],
            ),
            RequestSpec(
                method="HEAD",
                service="blob",
                path=f"/{ACCOUNT_NAME}/{self.container_name}/{blob_name}",
                query=[("snapshot", rust_snapshot)],
            ),
            comparison_spec,
        )
        return outcome_to_scenario("Blob snapshots with metadata", head_outcome)

    def scenario_list_blobs(self) -> ScenarioResult:
        spec = RequestSpec(
            method="GET",
            service="blob",
            path=f"/{ACCOUNT_NAME}/{self.container_name}",
            query=[("restype", "container"), ("comp", "list"), ("include", "metadata")],
            headers=[("Accept", "application/xml")],
            ignore_body_fields={"serviceendpoint"},
        )
        return outcome_to_scenario("List blobs", self.execute_and_compare(spec))

    def scenario_delete_blob(self) -> ScenarioResult:
        blob_name = "delete-me.txt"
        seed_spec = RequestSpec(
            method="PUT",
            service="blob",
            path=f"/{ACCOUNT_NAME}/{self.container_name}/{blob_name}",
            body=b"delete target",
            headers=[
                ("x-ms-blob-type", "BlockBlob"),
                ("Content-Type", "text/plain"),
            ],
        )
        seed_outcome = self.execute_and_compare(seed_spec)
        if not seed_outcome.ok:
            return outcome_to_scenario("Delete blob", seed_outcome)
        delete_spec = RequestSpec(
            method="DELETE",
            service="blob",
            path=f"/{ACCOUNT_NAME}/{self.container_name}/{blob_name}",
        )
        return outcome_to_scenario("Delete blob", self.execute_and_compare(delete_spec))

    def scenario_create_queue(self) -> ScenarioResult:
        spec = RequestSpec(
            method="PUT",
            service="queue",
            path=f"/{ACCOUNT_NAME}/{self.queue_name}",
        )
        return outcome_to_scenario("Create queue", self.execute_and_compare(spec))

    def scenario_put_message(self) -> ScenarioResult:
        body = (
            '<?xml version="1.0" encoding="utf-8"?>'
            "<QueueMessage><MessageText>hello queue</MessageText></QueueMessage>"
        ).encode("utf-8")
        spec = RequestSpec(
            method="POST",
            service="queue",
            path=f"/{ACCOUNT_NAME}/{self.queue_name}/messages",
            body=body,
            headers=[("Content-Type", "application/xml")],
        )
        return outcome_to_scenario("Put message", self.execute_and_compare(spec))

    def scenario_get_messages(self) -> ScenarioResult:
        spec = RequestSpec(
            method="GET",
            service="queue",
            path=f"/{ACCOUNT_NAME}/{self.queue_name}/messages",
            query=[("numofmessages", "1")],
            headers=[("Accept", "application/xml")],
        )
        return outcome_to_scenario("Get messages", self.execute_and_compare(spec))

    def scenario_delete_message(self) -> ScenarioResult:
        body = (
            '<?xml version="1.0" encoding="utf-8"?>'
            "<QueueMessage><MessageText>delete me</MessageText></QueueMessage>"
        ).encode("utf-8")
        put_spec = RequestSpec(
            method="POST",
            service="queue",
            path=f"/{ACCOUNT_NAME}/{self.queue_name}/messages",
            body=body,
            headers=[("Content-Type", "application/xml")],
        )
        put_outcome, ts_response, rust_response = self.execute_variant_compare(put_spec, put_spec)
        if not put_outcome.ok:
            return outcome_to_scenario("Delete message", put_outcome)
        ts_message_id = extract_xml_text(ts_response.body, "MessageId")
        rust_message_id = extract_xml_text(rust_response.body, "MessageId")
        ts_pop_receipt = extract_xml_text(ts_response.body, "PopReceipt")
        rust_pop_receipt = extract_xml_text(rust_response.body, "PopReceipt")
        if not all([ts_message_id, rust_message_id, ts_pop_receipt, rust_pop_receipt]):
            return ScenarioResult(
                name="Delete message",
                ok=False,
                summary="Queue delete identifiers missing from send response",
                details=[
                    f"TS MessageId={ts_message_id or '<missing>'} PopReceipt={ts_pop_receipt or '<missing>'}",
                    f"Rust MessageId={rust_message_id or '<missing>'} PopReceipt={rust_pop_receipt or '<missing>'}",
                ],
            )
        comparison_spec = RequestSpec(
            method="DELETE",
            service="queue",
            path=f"/{ACCOUNT_NAME}/{self.queue_name}/messages/message-id",
            query=[("popreceipt", "DYNAMIC")],
        )
        delete_outcome, _ts_delete, _rust_delete = self.execute_variant_compare(
            RequestSpec(
                method="DELETE",
                service="queue",
                path=(
                    f"/{ACCOUNT_NAME}/{self.queue_name}/messages/"
                    f"{urllib.parse.quote(ts_message_id, safe='')}"
                ),
                query=[("popreceipt", ts_pop_receipt)],
            ),
            RequestSpec(
                method="DELETE",
                service="queue",
                path=(
                    f"/{ACCOUNT_NAME}/{self.queue_name}/messages/"
                    f"{urllib.parse.quote(rust_message_id, safe='')}"
                ),
                query=[("popreceipt", rust_pop_receipt)],
            ),
            comparison_spec,
        )
        return outcome_to_scenario("Delete message", delete_outcome)

    def scenario_delete_queue(self) -> ScenarioResult:
        spec = RequestSpec(
            method="DELETE",
            service="queue",
            path=f"/{ACCOUNT_NAME}/{self.queue_name}",
        )
        return outcome_to_scenario("Delete queue", self.execute_and_compare(spec))

    def scenario_create_table(self) -> ScenarioResult:
        body = json.dumps({"TableName": self.table_name}, separators=(",", ":")).encode("utf-8")
        spec = RequestSpec(
            method="POST",
            service="table",
            path=f"/{ACCOUNT_NAME}/Tables",
            body=body,
            headers=[
                ("Accept", "application/json;odata=nometadata"),
                ("Content-Type", "application/json"),
            ],
        )
        return outcome_to_scenario("Create table", self.execute_and_compare(spec))

    def scenario_insert_entity(self) -> ScenarioResult:
        entity = {
            "PartitionKey": "pk",
            "RowKey": "rk",
            "DisplayName": "Boromir",
            "Score": 42,
        }
        body = json.dumps(entity, separators=(",", ":")).encode("utf-8")
        spec = RequestSpec(
            method="POST",
            service="table",
            path=f"/{ACCOUNT_NAME}/{self.table_name}",
            body=body,
            headers=[
                ("Accept", "application/json;odata=nometadata"),
                ("Content-Type", "application/json"),
            ],
            ignore_body_fields={"lastModifiedTime"},
        )
        return outcome_to_scenario("Insert entity", self.execute_and_compare(spec))

    def scenario_get_entity(self) -> ScenarioResult:
        spec = RequestSpec(
            method="GET",
            service="table",
            path=f"/{ACCOUNT_NAME}/{self.table_name}(PartitionKey='pk',RowKey='rk')",
            headers=[("Accept", "application/json;odata=nometadata")],
            ignore_body_fields={"lastModifiedTime"},
        )
        return outcome_to_scenario("Get entity", self.execute_and_compare(spec))

    def scenario_query_entities(self) -> ScenarioResult:
        spec = RequestSpec(
            method="GET",
            service="table",
            path=f"/{ACCOUNT_NAME}/{self.table_name}()",
            query=[("$filter", "PartitionKey eq 'pk'")],
            headers=[("Accept", "application/json;odata=nometadata")],
            ignore_body_fields={"lastModifiedTime"},
        )
        return outcome_to_scenario("Query entities", self.execute_and_compare(spec))

    def scenario_delete_entity(self) -> ScenarioResult:
        spec = RequestSpec(
            method="DELETE",
            service="table",
            path=f"/{ACCOUNT_NAME}/{self.table_name}(PartitionKey='pk',RowKey='rk')",
            headers=[("If-Match", "*")],
        )
        return outcome_to_scenario("Delete entity", self.execute_and_compare(spec))

def outcome_to_scenario(name: str, outcome: ComparisonOutcome) -> ScenarioResult:
    return ScenarioResult(name=name, ok=outcome.ok, summary=outcome.summary, details=outcome.details)


def format_rfc1123(moment: dt.datetime) -> str:
    return moment.strftime("%a, %d %b %Y %H:%M:%S GMT")


def build_authorization(spec: RequestSpec, headers: list[tuple[str, str]], body: bytes) -> str:
    if spec.service == "table":
        return build_table_authorization(spec, headers)
    return build_blob_queue_authorization(spec, headers, body)


def build_blob_queue_authorization(
    spec: RequestSpec, headers: list[tuple[str, str]], body: bytes
) -> str:
    header_map = ordered_header_map(headers)
    string_to_sign = "\n".join(
        [
            spec.method.upper(),
            first_header_value(header_map, "content-encoding"),
            first_header_value(header_map, "content-language"),
            content_length_for_signature(header_map, body),
            first_header_value(header_map, "content-md5"),
            first_header_value(header_map, "content-type"),
            first_header_value(header_map, "date"),
            first_header_value(header_map, "if-modified-since"),
            first_header_value(header_map, "if-match"),
            first_header_value(header_map, "if-none-match"),
            first_header_value(header_map, "if-unmodified-since"),
            first_header_value(header_map, "range"),
            canonicalized_x_ms_headers(header_map),
            canonicalized_resource(spec.path, spec.query),
        ]
    )
    digest = hmac.new(
        base64.b64decode(ACCOUNT_KEY),
        string_to_sign.encode("utf-8"),
        hashlib.sha256,
    ).digest()
    signature = base64.b64encode(digest).decode("ascii")
    return f"SharedKey {ACCOUNT_NAME}:{signature}"


def build_table_authorization(spec: RequestSpec, headers: list[tuple[str, str]]) -> str:
    header_map = ordered_header_map(headers)
    date_value = first_header_value(header_map, "date") or first_header_value(
        header_map, "x-ms-date"
    )
    string_to_sign = f"{date_value}\n/{ACCOUNT_NAME}{spec.path}"
    if any(key.lower() == "comp" for key, _value in spec.query):
        comp_value = next(value for key, value in spec.query if key.lower() == "comp")
        string_to_sign += f"?comp={comp_value}"
    digest = hmac.new(
        base64.b64decode(ACCOUNT_KEY),
        string_to_sign.encode("utf-8"),
        hashlib.sha256,
    ).digest()
    signature = base64.b64encode(digest).decode("ascii")
    return f"SharedKeyLite {ACCOUNT_NAME}:{signature}"


def content_length_for_signature(
    header_map: dict[str, list[str]], body: bytes
) -> str:
    value = first_header_value(header_map, "content-length")
    if not value:
        return ""
    try:
        numeric = int(value)
    except ValueError:
        return value
    return "" if numeric == 0 else str(numeric)


def ordered_header_map(headers: list[tuple[str, str]]) -> dict[str, list[str]]:
    result: dict[str, list[str]] = {}
    for name, value in headers:
        result.setdefault(name.lower(), []).append(normalize_header_whitespace(value))
    return result


def first_header_value(header_map: dict[str, list[str]], name: str) -> str:
    values = header_map.get(name.lower())
    return values[0] if values else ""


def canonicalized_x_ms_headers(header_map: dict[str, list[str]]) -> str:
    parts = []
    for name in sorted(k for k in header_map.keys() if k.startswith("x-ms-")):
        parts.append(f"{name}:{','.join(header_map[name])}")
    return "\n".join(parts)


def canonicalized_resource(path: str, query: list[tuple[str, str]]) -> str:
    base = f"/{ACCOUNT_NAME}{path}"
    if not query:
        return base
    grouped: dict[str, list[str]] = {}
    for key, value in query:
        grouped.setdefault(key.lower(), []).append(urllib.parse.unquote(str(value)))
    lines = [base]
    for key in sorted(grouped):
        lines.append(f"{key}:{','.join(sorted(grouped[key]))}")
    return "\n".join(lines)


def normalize_header_whitespace(value: str) -> str:
    return " ".join(value.strip().split())


def normalize_header_value(name: str, value: str) -> str:
    normalized = normalize_header_whitespace(value)
    return DYNAMIC_HEADER_PLACEHOLDERS.get(name.lower(), normalized)


def compare_responses(
    ts_response: HttpResponseData,
    rust_response: HttpResponseData,
    spec: RequestSpec,
) -> ComparisonOutcome:
    details: list[str] = []
    if ts_response.status != rust_response.status:
        details.append(
            f"Status: {ts_response.status} vs {rust_response.status}"
        )
    body_result = compare_body(ts_response.body, rust_response.body, ts_response.headers, rust_response.headers, spec)
    header_result = compare_headers(ts_response, rust_response)
    if not body_result:
        header_result = [line for line in header_result if not line.strip().startswith("content-length:")]
    if header_result:
        details.append("Headers differ:")
        details.extend(header_result)
    if body_result:
        details.append(body_result[0])
        details.extend(body_result[1:])
    if details:
        return ComparisonOutcome(ok=False, summary=details[0], details=details)
    summary = build_pass_summary(ts_response, rust_response, spec)
    return ComparisonOutcome(ok=True, summary=summary)


def compare_headers(
    ts_response: HttpResponseData, rust_response: HttpResponseData
) -> list[str]:
    ts_map = header_set(ts_response.headers, len(ts_response.body))
    rust_map = header_set(rust_response.headers, len(rust_response.body))
    lines: list[str] = []
    ts_keys = set(ts_map)
    rust_keys = set(rust_map)
    if ts_keys != rust_keys:
        if ts_keys - rust_keys:
            lines.append(f"  Rust missing headers: {sorted(ts_keys - rust_keys)}")
        if rust_keys - ts_keys:
            lines.append(f"  TS missing headers: {sorted(rust_keys - ts_keys)}")
    for key in sorted(ts_keys & rust_keys):
        if ts_map[key] != rust_map[key]:
            lines.append(
                f"  {key}: TS={ts_map.get(key, [])} Rust={rust_map.get(key, [])}"
            )
    return lines


def header_set(headers: list[tuple[str, str]], body_length: int = 0) -> dict[str, list[str]]:
    result: dict[str, list[str]] = {}
    saw_chunked = False
    for name, value in headers:
        lower = name.lower()
        if lower == "transfer-encoding" and "chunked" in value.lower():
            saw_chunked = True
        if lower in TRANSPORT_IGNORED_HEADER_NAMES:
            continue
        result.setdefault(lower, []).append(normalize_header_value(lower, value))
    if "content-length" not in result and saw_chunked:
        result["content-length"] = [str(body_length)]
    return result


def compare_body(
    ts_body: bytes,
    rust_body: bytes,
    ts_headers: list[tuple[str, str]],
    rust_headers: list[tuple[str, str]],
    spec: RequestSpec,
) -> list[str]:
    if not ts_body and not rust_body:
        return []
    body_kind = determine_body_kind(ts_body, rust_body, ts_headers, rust_headers, spec.expect)
    ignored_fields = {normalize_field_name(name) for name in spec.ignore_body_fields}
    if body_kind == "json":
        try:
            ts_normalized = normalize_json(json.loads(ts_body.decode("utf-8")), ignored_fields)
            rust_normalized = normalize_json(json.loads(rust_body.decode("utf-8")), ignored_fields)
        except Exception as exc:
            return [f"Body: JSON parse failed ({exc})"]
        if ts_normalized != rust_normalized:
            return ["Body differs (JSON):", *render_diff(ts_normalized, rust_normalized)]
        return []
    if body_kind == "xml":
        try:
            ts_normalized = normalize_xml(ET.fromstring(ts_body), ignored_fields)
            rust_normalized = normalize_xml(ET.fromstring(rust_body), ignored_fields)
        except Exception as exc:
            return [f"Body: XML parse failed ({exc})"]
        if ts_normalized != rust_normalized:
            return ["Body differs (XML):", *render_diff(ts_normalized, rust_normalized)]
        return []
    if ts_body != rust_body:
        return [
            "Body differs (binary):",
            f"  TS bytes={len(ts_body)} sha256={hashlib.sha256(ts_body).hexdigest()}",
            f"  Rust bytes={len(rust_body)} sha256={hashlib.sha256(rust_body).hexdigest()}",
        ]
    return []


def determine_body_kind(
    ts_body: bytes,
    rust_body: bytes,
    ts_headers: list[tuple[str, str]],
    rust_headers: list[tuple[str, str]],
    expected: str,
) -> str:
    if expected != "auto":
        return expected
    content_types = []
    for headers in (ts_headers, rust_headers):
        for name, value in headers:
            if name.lower() == "content-type":
                content_types.append(value.lower())
    combined = " ".join(content_types)
    if "json" in combined:
        return "json"
    if "xml" in combined:
        return "xml"
    stripped = next((body.lstrip() for body in (ts_body, rust_body) if body.strip()), b"")
    if stripped.startswith((b"{", b"[")):
        return "json"
    if stripped.startswith(b"<"):
        return "xml"
    return "binary"


def normalize_json(value: object, ignored_fields: set[str], field_name: Optional[str] = None) -> object:
    normalized_field_name = normalize_field_name(field_name or "")
    placeholder = DYNAMIC_FIELD_PLACEHOLDERS.get(normalized_field_name)
    if placeholder is not None:
        return placeholder
    if isinstance(value, dict):
        normalized = {}
        for key, child in value.items():
            child_field_name = normalize_field_name(key)
            if child_field_name in ignored_fields:
                continue
            normalized[key] = normalize_json(child, ignored_fields, child_field_name)
        return normalized
    if isinstance(value, list):
        return [normalize_json(item, ignored_fields, field_name) for item in value]
    if isinstance(value, str):
        return normalize_dynamic_text(normalized_field_name, value)
    return value


def normalize_xml(element: ET.Element, ignored_fields: set[str]) -> tuple[str, tuple[tuple[str, str], ...], Optional[str], list[object]]:
    name = normalize_xml_name(element.tag)
    normalized_name = normalize_field_name(name)
    attributes = []
    for attr_name, attr_value in sorted(element.attrib.items()):
        normalized_attr_name = normalize_field_name(attr_name)
        if normalized_attr_name in ignored_fields:
            continue
        attributes.append((normalize_xml_name(attr_name), normalize_dynamic_text(normalized_attr_name, attr_value.strip())))
    raw_text = (element.text or "").strip()
    text = normalize_dynamic_text(normalized_name, raw_text) if raw_text else None
    children = []
    for child in list(element):
        child_name = normalize_xml_name(child.tag)
        if normalize_field_name(child_name) in ignored_fields:
            continue
        children.append(normalize_xml(child, ignored_fields))
    children.sort(key=lambda item: json.dumps(item, sort_keys=True, ensure_ascii=False))
    return name, tuple(attributes), text, children


def normalize_xml_name(name: str) -> str:
    if "}" in name:
        return name.split("}", 1)[1]
    return name


def normalize_field_name(name: str) -> str:
    return "".join(ch for ch in normalize_xml_name(name).lower() if ch.isalnum() or ch == ".")


def normalize_dynamic_text(field_name: str, value: str) -> str:
    placeholder = DYNAMIC_FIELD_PLACEHOLDERS.get(field_name)
    if placeholder is not None:
        return placeholder
    normalized = value.strip()
    normalized = REQUEST_ID_MESSAGE_RE.sub("RequestId: DYNAMIC", normalized)
    normalized = TIME_MESSAGE_RE.sub("Time: DYNAMIC", normalized)
    if field_name.endswith("etag") or looks_like_etag(normalized):
        return "DYNAMIC_ETAG"
    return normalized


def looks_like_etag(value: str) -> bool:
    return bool(
        HEX_ETAG_RE.fullmatch(value)
        or QUOTED_HEX_ETAG_RE.fullmatch(value)
        or WEAK_ETAG_RE.fullmatch(value)
    )


def extract_xml_text(body: bytes, tag_name: str) -> str:
    normalized_tag = normalize_field_name(tag_name)
    root = ET.fromstring(body)
    for element in root.iter():
        if normalize_field_name(element.tag) == normalized_tag:
            return (element.text or "").strip()
    return ""

def render_diff(left: object, right: object) -> list[str]:
    left_dump = json.dumps(left, indent=2, sort_keys=True, ensure_ascii=False).splitlines()
    right_dump = json.dumps(right, indent=2, sort_keys=True, ensure_ascii=False).splitlines()
    diff = list(difflib.unified_diff(left_dump, right_dump, fromfile="ts", tofile="rust", lineterm=""))
    if not diff:
        return ["  Values differ but no textual diff was generated."]
    return [f"  {line}" for line in diff[:80]]


def build_pass_summary(
    ts_response: HttpResponseData,
    rust_response: HttpResponseData,
    spec: RequestSpec,
) -> str:
    header_count = len(header_set(ts_response.headers, len(ts_response.body)))
    body_kind = determine_body_kind(ts_response.body, rust_response.body, ts_response.headers, rust_response.headers, spec.expect)
    if not ts_response.body and not rust_response.body:
        body_summary = "empty=empty"
    elif body_kind == "json":
        body_summary = "JSON matches"
    elif body_kind == "xml":
        body_summary = "XML matches"
    else:
        body_summary = "binary matches"
    return (
        f"status: {ts_response.status}={rust_response.status}, "
        f"headers: {header_count}/{header_count} match, "
        f"body: {body_summary}"
    )


def print_results(results: list[ScenarioResult], artifacts_dir: pathlib.Path) -> int:
    print("Differential Test Results")
    print("=========================")
    failures = 0
    for result in results:
        if result.ok:
            print(f"✅ PASS: {result.name} ({result.summary})")
            continue
        failures += 1
        print(f"❌ FAIL: {result.name}")
        for detail in result.details or [result.summary]:
            print(f"  {detail}")
    passed = len(results) - failures
    print()
    print(f"Summary: {passed} passed, {failures} failed")
    print(f"Artifacts: {artifacts_dir}")
    return failures


def parse_args() -> argparse.Namespace:
    parser = argparse.ArgumentParser(description="Run TypeScript vs Rust Azurite differential tests.")
    parser.add_argument(
        "--keep-artifacts",
        action="store_true",
        help="Keep server logs and state directories instead of deleting the temp directory.",
    )
    parser.add_argument(
        "--artifacts-dir",
        help="Write logs and temporary state to this directory.",
    )
    return parser.parse_args()


def main() -> int:
    args = parse_args()
    harness = DifferentialHarness(
        keep_artifacts=args.keep_artifacts or bool(args.artifacts_dir),
        artifacts_dir=args.artifacts_dir,
    )
    try:
        harness.start_servers()
        results = harness.run()
        return 1 if print_results(results, harness.artifacts_dir) else 0
    finally:
        harness.cleanup()


if __name__ == "__main__":
    sys.exit(main())
