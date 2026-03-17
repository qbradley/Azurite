#!/usr/bin/env python3
"""
Swagger-Based Differential Test Runner

Sends identical requests to both TS Azurite and Rust Azurite,
compares responses to find behavioral differences.

Usage:
    python3 -m swagger_diff.runner --ts-port 10001 --rust-port 10000
    python3 -m swagger_diff.runner --operations Container_Create,Blob_Download
    python3 -m swagger_diff.runner --tier 0  # Only tier-0 (no prerequisites)
    python3 -m swagger_diff.runner --group Container  # Only container operations
"""

import argparse
import http.client
import json
import pathlib
import re
import sys
import time
import urllib.parse
import xml.etree.ElementTree as ET
from dataclasses import dataclass, field
from typing import Any, Optional

try:
    from .swagger_parser import SwaggerSpec, Operation, RequestBuilder
    HAVE_PARSER = True
except ImportError:
    HAVE_PARSER = False

TRANSPORT_IGNORED_HEADER_NAMES = {
    "connection",
    "content-length",
    "keep-alive",
    "server",
    "transfer-encoding",
}

DYNAMIC_HEADER_PLACEHOLDERS = {
    "date": "DYNAMIC_DATE",
    "etag": "DYNAMIC_ETAG",
    "last-modified": "DYNAMIC_TIMESTAMP",
    "x-ms-access-tier-change-time": "DYNAMIC_TIMESTAMP",
    "x-ms-copy-completion-time": "DYNAMIC_TIMESTAMP",
    "x-ms-copy-id": "DYNAMIC_COPY_ID",
    "x-ms-copy-source": "DYNAMIC_COPY_SOURCE",
    "x-ms-creation-time": "DYNAMIC_TIMESTAMP",
    "x-ms-expiry-time": "DYNAMIC_TIMESTAMP",
    "x-ms-immutability-policy-until-date": "DYNAMIC_TIMESTAMP",
    "x-ms-lease-id": "DYNAMIC_LEASE_ID",
    "x-ms-lease-time": "DYNAMIC_LEASE_TIME",
    "x-ms-request-id": "DYNAMIC_REQUEST_ID",
    "x-ms-snapshot": "DYNAMIC_SNAPSHOT",
    "x-ms-version-id": "DYNAMIC_VERSION_ID",
}

DYNAMIC_FIELD_PLACEHOLDERS = {
    "accesstierchangetime": "DYNAMIC_TIMESTAMP",
    "clientrequestid": "DYNAMIC_REQUEST_ID",
    "contentmd5": "DYNAMIC_CONTENT_MD5",
    "copycompletiontime": "DYNAMIC_TIMESTAMP",
    "copyid": "DYNAMIC_COPY_ID",
    "copyprogress": "DYNAMIC_COPY_PROGRESS",
    "copysource": "DYNAMIC_COPY_SOURCE",
    "creationtime": "DYNAMIC_TIMESTAMP",
    "date": "DYNAMIC_DATE",
    "deletedon": "DYNAMIC_TIMESTAMP",
    "etag": "DYNAMIC_ETAG",
    "expirationtime": "DYNAMIC_TIMESTAMP",
    "insertiontime": "DYNAMIC_TIMESTAMP",
    "lastmodified": "DYNAMIC_TIMESTAMP",
    "lastmodifiedtime": "DYNAMIC_TIMESTAMP",
    "lastsynctime": "DYNAMIC_TIMESTAMP",
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


@dataclass
class ScenarioResult:
    """Result of one differential test scenario"""
    operation_id: str
    scenario_name: str
    ts_status: int
    rust_status: int
    passed: bool
    differences: list[str] = field(default_factory=list)
    setup_divergence: bool = False
    error: Optional[str] = None


@dataclass
class Scenario:
    """Test scenario for an operation"""
    name: str
    overrides: dict[str, Any]
    expected_success: bool = True


class ComparisonEngine:
    """Compares two HTTP responses for equivalence"""
    
    def __init__(self, verbose: bool = False):
        self.verbose = verbose
    
    def compare(self, ts_response: dict[str, Any], rust_response: dict[str, Any]) -> tuple[bool, list[str]]:
        """Compare status, headers, and body. Returns (passed, differences)"""
        differences = []
        
        ts_status = ts_response.get("status_code", 0)
        rust_status = rust_response.get("status_code", 0)
        
        if ts_status != rust_status:
            differences.append(f"Status code: TS={ts_status} != Rust={rust_status}")
        
        header_diff = self._compare_headers(
            ts_response.get("headers", {}),
            rust_response.get("headers", {})
        )
        if header_diff:
            differences.append("Headers differ:")
            differences.extend(header_diff)
        
        body_diff = self._compare_bodies(
            ts_response.get("body", b""),
            rust_response.get("body", b""),
            ts_response.get("headers", {}),
            rust_response.get("headers", {})
        )
        if body_diff:
            differences.append("Body differs:")
            differences.extend(body_diff)
        
        return (len(differences) == 0, differences)
    
    def _compare_headers(self, ts_headers: dict[str, str], rust_headers: dict[str, str]) -> list[str]:
        """Compare headers with normalization"""
        ts_norm = self._normalize_headers(ts_headers)
        rust_norm = self._normalize_headers(rust_headers)
        
        differences = []
        
        ts_keys = set(ts_norm.keys())
        rust_keys = set(rust_norm.keys())
        
        missing_in_rust = ts_keys - rust_keys
        extra_in_rust = rust_keys - ts_keys
        
        if missing_in_rust:
            differences.append(f"  Missing in Rust: {sorted(missing_in_rust)}")
        if extra_in_rust:
            differences.append(f"  Extra in Rust: {sorted(extra_in_rust)}")
        
        for key in sorted(ts_keys & rust_keys):
            if ts_norm[key] != rust_norm[key]:
                differences.append(f"  {key}: TS={ts_norm[key]} != Rust={rust_norm[key]}")
        
        return differences
    
    def _normalize_headers(self, headers: dict[str, str]) -> dict[str, str]:
        """Normalize headers for comparison"""
        normalized = {}
        
        for name, value in headers.items():
            lower_name = name.lower()
            
            if lower_name in TRANSPORT_IGNORED_HEADER_NAMES:
                continue
            
            normalized_value = DYNAMIC_HEADER_PLACEHOLDERS.get(lower_name, value.strip())
            normalized[lower_name] = normalized_value
        
        return normalized
    
    def _compare_bodies(
        self,
        ts_body: bytes,
        rust_body: bytes,
        ts_headers: dict[str, str],
        rust_headers: dict[str, str]
    ) -> list[str]:
        """Compare bodies with content-type aware normalization"""
        if not ts_body and not rust_body:
            return []
        
        content_type = self._detect_content_type(ts_headers, rust_headers, ts_body, rust_body)
        
        if content_type == "json":
            return self._compare_json_bodies(ts_body, rust_body)
        elif content_type == "xml":
            return self._compare_xml_bodies(ts_body, rust_body)
        else:
            return self._compare_binary_bodies(ts_body, rust_body)
    
    def _detect_content_type(
        self,
        ts_headers: dict[str, str],
        rust_headers: dict[str, str],
        ts_body: bytes,
        rust_body: bytes
    ) -> str:
        """Detect content type for body comparison"""
        content_types = []
        
        for headers in [ts_headers, rust_headers]:
            ct = headers.get("Content-Type", headers.get("content-type", "")).lower()
            content_types.append(ct)
        
        combined = " ".join(content_types)
        
        if "json" in combined:
            return "json"
        if "xml" in combined:
            return "xml"
        
        sample = next((body.lstrip() for body in [ts_body, rust_body] if body.strip()), b"")
        if sample.startswith((b"{", b"[")):
            return "json"
        if sample.startswith(b"<"):
            return "xml"
        
        return "binary"
    
    def _compare_json_bodies(self, ts_body: bytes, rust_body: bytes) -> list[str]:
        """Compare JSON bodies with normalization"""
        try:
            ts_obj = json.loads(ts_body.decode("utf-8"))
            rust_obj = json.loads(rust_body.decode("utf-8"))
        except Exception as e:
            return [f"  JSON parse failed: {e}"]
        
        ts_norm = self._normalize_json(ts_obj)
        rust_norm = self._normalize_json(rust_obj)
        
        if ts_norm == rust_norm:
            return []
        
        return [
            "  JSON structures differ",
            f"  TS: {json.dumps(ts_norm, indent=2)[:500]}",
            f"  Rust: {json.dumps(rust_norm, indent=2)[:500]}",
        ]
    
    def _normalize_json(self, value: Any, field_name: str = "") -> Any:
        """Normalize JSON value recursively"""
        normalized_field = self._normalize_field_name(field_name)
        placeholder = DYNAMIC_FIELD_PLACEHOLDERS.get(normalized_field)
        
        if placeholder is not None:
            return placeholder
        
        if isinstance(value, dict):
            normalized = {}
            for key, child in value.items():
                normalized[key] = self._normalize_json(child, key)
            return normalized
        
        if isinstance(value, list):
            return [self._normalize_json(item, field_name) for item in value]
        
        if isinstance(value, str):
            return self._normalize_dynamic_text(normalized_field, value)
        
        return value
    
    def _compare_xml_bodies(self, ts_body: bytes, rust_body: bytes) -> list[str]:
        """Compare XML bodies with normalization"""
        try:
            ts_tree = ET.fromstring(ts_body)
            rust_tree = ET.fromstring(rust_body)
        except Exception as e:
            return [f"  XML parse failed: {e}"]
        
        ts_norm = self._normalize_xml(ts_tree)
        rust_norm = self._normalize_xml(rust_tree)
        
        if ts_norm == rust_norm:
            return []
        
        return [
            "  XML structures differ",
            f"  TS: {json.dumps(ts_norm, indent=2)[:500]}",
            f"  Rust: {json.dumps(rust_norm, indent=2)[:500]}",
        ]
    
    def _normalize_xml(self, element: ET.Element) -> tuple:
        """Normalize XML element recursively"""
        name = self._normalize_xml_name(element.tag)
        normalized_name = self._normalize_field_name(name)
        
        attributes = []
        for attr_name, attr_value in sorted(element.attrib.items()):
            normalized_attr_name = self._normalize_field_name(attr_name)
            norm_value = self._normalize_dynamic_text(normalized_attr_name, attr_value.strip())
            attributes.append((self._normalize_xml_name(attr_name), norm_value))
        
        raw_text = (element.text or "").strip()
        text = self._normalize_dynamic_text(normalized_name, raw_text) if raw_text else None
        
        children = []
        for child in list(element):
            children.append(self._normalize_xml(child))
        
        children.sort(key=lambda item: json.dumps(item, sort_keys=True, ensure_ascii=False))
        
        return (name, tuple(attributes), text, tuple(children))
    
    def _normalize_xml_name(self, name: str) -> str:
        """Normalize XML element/attribute name"""
        if "}" in name:
            return name.split("}", 1)[1]
        return name
    
    def _normalize_field_name(self, name: str) -> str:
        """Normalize field name for lookup"""
        return "".join(ch for ch in self._normalize_xml_name(name).lower() if ch.isalnum() or ch == ".")
    
    def _normalize_dynamic_text(self, field_name: str, value: str) -> str:
        """Normalize dynamic text content"""
        placeholder = DYNAMIC_FIELD_PLACEHOLDERS.get(field_name)
        if placeholder is not None:
            return placeholder
        
        normalized = value.strip()
        normalized = REQUEST_ID_MESSAGE_RE.sub("RequestId: DYNAMIC", normalized)
        normalized = TIME_MESSAGE_RE.sub("Time: DYNAMIC", normalized)
        
        if field_name.endswith("etag") or self._looks_like_etag(normalized):
            return "DYNAMIC_ETAG"
        
        return normalized
    
    def _looks_like_etag(self, value: str) -> bool:
        """Check if value looks like an ETag"""
        return bool(
            HEX_ETAG_RE.fullmatch(value)
            or QUOTED_HEX_ETAG_RE.fullmatch(value)
            or WEAK_ETAG_RE.fullmatch(value)
        )
    
    def _compare_binary_bodies(self, ts_body: bytes, rust_body: bytes) -> list[str]:
        """Compare binary bodies"""
        if ts_body == rust_body:
            return []
        
        return [
            f"  Binary bodies differ: TS={len(ts_body)} bytes, Rust={len(rust_body)} bytes"
        ]


class ScenarioGenerator:
    """Generates test scenarios for an operation"""
    
    def generate(self, operation: Any) -> list[Scenario]:
        """Generate scenarios for this operation.
        
        Phase 1: Just happy-path — valid request with all required params.
        Phase 2 (later): Missing required params, invalid values, boundary values.
        """
        scenarios = []
        scenarios.append(Scenario(
            name="happy_path",
            overrides={},
            expected_success=True,
        ))
        return scenarios


class DifferentialRunner:
    """Runs scenarios against both servers"""
    
    def __init__(
        self,
        ts_host: str,
        ts_port: int,
        rust_host: str,
        rust_port: int,
        spec: Any,
        verbose: bool = False,
        timeout: int = 30
    ):
        self.ts_host = ts_host
        self.ts_port = ts_port
        self.rust_host = rust_host
        self.rust_port = rust_port
        self.spec = spec
        self.verbose = verbose
        self.timeout = timeout
        self.comparison_engine = ComparisonEngine(verbose=verbose)
        self.scenario_generator = ScenarioGenerator()
    
    def check_servers(self) -> bool:
        """Verify both servers are reachable"""
        ts_ok = self._check_server(self.ts_host, self.ts_port, "TypeScript")
        rust_ok = self._check_server(self.rust_host, self.rust_port, "Rust")
        return ts_ok and rust_ok
    
    def _check_server(self, host: str, port: int, name: str) -> bool:
        """Check if a server is reachable"""
        try:
            # Use authenticated request for emulator
            acct = "devstoreaccount1"
            status, _, _ = self._make_auth_request(host, port, "GET", f"/{acct}", [("comp", "list")])
            print(f"✓ {name} Azurite reachable at {host}:{port} (status: {status})", flush=True)
            return True
        except Exception as e:
            print(f"✗ {name} Azurite NOT reachable at {host}:{port}: {e}", file=sys.stderr)
            return False
    
    def run_operation(self, operation_id: str) -> list[ScenarioResult]:
        """Run all scenarios for one operation"""
        if self.verbose:
            print(f"\n=== Running operation: {operation_id} ===", flush=True)
        
        operation = self.spec.get_operation(operation_id)
        if not operation:
            return [ScenarioResult(
                operation_id=operation_id,
                scenario_name="N/A",
                ts_status=0,
                rust_status=0,
                passed=False,
                error=f"Operation {operation_id} not found in spec"
            )]
        
        scenarios = self.scenario_generator.generate(operation)
        results = []
        
        for scenario in scenarios:
            result = self._run_scenario(operation, scenario)
            results.append(result)
        
        self.reset_state()
        
        return results
    
    def _run_scenario(self, operation: Any, scenario: Scenario) -> ScenarioResult:
        """Execute one scenario against both servers, including setup chain"""
        import random, string
        
        try:
            # Generate shared resource names so both servers get identical requests
            suffix = "".join(random.choices(string.ascii_lowercase + string.digits, k=8))
            container_name = f"tc{suffix}"
            blob_name = f"tb{suffix}"
            shared_overrides = {
                "_containerName": container_name,
                "_blob": blob_name,
            }
            shared_overrides.update(scenario.overrides)
            
            account_name = "devstoreaccount1"
            account_key = "Eby8vdM02xNOcqFlqUwJPLlmEtlCDXJ1OUzFT50uSRZ6IFsuFq2UVErCz4I6tq/K1SZFPTOtr/KBHBeksoGMGw=="
            
            ts_builder = RequestBuilder(
                account_name=account_name, account_key=account_key,
                host=self.ts_host, port=self.ts_port
            )
            rust_builder = RequestBuilder(
                account_name=account_name, account_key=account_key,
                host=self.rust_host, port=self.rust_port
            )
            
            # Execute setup chain (dependency operations that must succeed first)
            setup_chain = self.spec.get_dependency_chain(operation.operation_id)
            for setup_op in setup_chain:
                setup_overrides = dict(shared_overrides)
                # PageBlob_Create needs special headers
                if setup_op.operation_id == "PageBlob_Create":
                    setup_overrides["x-ms-blob-content-length"] = "512"
                
                ts_setup_req = ts_builder.build_request(setup_op, setup_overrides)
                rust_setup_req = rust_builder.build_request(setup_op, setup_overrides)
                
                ts_setup_resp = self._send_request_obj(ts_setup_req)
                rust_setup_resp = self._send_request_obj(rust_setup_req)
                
                ts_setup_status = ts_setup_resp.get("status_code", 0)
                rust_setup_status = rust_setup_resp.get("status_code", 0)
                
                if self.verbose:
                    print(f"  Setup: {setup_op.operation_id} → TS={ts_setup_status}, Rust={rust_setup_status}", flush=True)
                
                # If setup diverges, that's a finding too
                if ts_setup_status != rust_setup_status:
                    return ScenarioResult(
                        operation_id=operation.operation_id,
                        scenario_name=scenario.name,
                        ts_status=ts_setup_status,
                        rust_status=rust_setup_status,
                        passed=False,
                        differences=[
                            f"Setup divergence in {setup_op.operation_id}: TS={ts_setup_status}, Rust={rust_setup_status}"
                        ],
                        setup_divergence=True
                    )
                
                # If setup failed on both, skip this scenario
                if ts_setup_status >= 400:
                    return ScenarioResult(
                        operation_id=operation.operation_id,
                        scenario_name=scenario.name,
                        ts_status=0,
                        rust_status=0,
                        passed=False,
                        error=f"Setup {setup_op.operation_id} failed on both servers: status {ts_setup_status}"
                    )
                
                # Capture lease-id from setup if it was a lease acquisition
                if "AcquireLease" in setup_op.operation_id:
                    ts_lease_id = ts_setup_resp.get("headers", {}).get("x-ms-lease-id", "")
                    rust_lease_id = rust_setup_resp.get("headers", {}).get("x-ms-lease-id", "")
                    # Use TS lease-id for TS requests and Rust lease-id for Rust requests
                    # We'll need to handle this specially
                    shared_overrides["_ts_lease_id"] = ts_lease_id
                    shared_overrides["_rust_lease_id"] = rust_lease_id
                    shared_overrides["x-ms-lease-id"] = ts_lease_id  # default for building
            
            # Now execute the actual target operation
            ts_request = ts_builder.build_request(operation, shared_overrides)
            
            # For Rust, swap lease-id if needed
            rust_overrides = dict(shared_overrides)
            if "_rust_lease_id" in rust_overrides:
                rust_overrides["x-ms-lease-id"] = rust_overrides["_rust_lease_id"]
            rust_request = rust_builder.build_request(operation, rust_overrides)
            
            if self.verbose:
                print(f"  Scenario: {scenario.name}", flush=True)
                print(f"    {ts_request.method} {ts_request.path}", flush=True)
            
            ts_response = self._send_request_obj(ts_request)
            rust_response = self._send_request_obj(rust_request)
            
            passed, differences = self.comparison_engine.compare(ts_response, rust_response)
            
            return ScenarioResult(
                operation_id=operation.operation_id,
                scenario_name=scenario.name,
                ts_status=ts_response.get("status_code", 0),
                rust_status=rust_response.get("status_code", 0),
                passed=passed,
                differences=differences
            )
            
        except Exception as e:
            return ScenarioResult(
                operation_id=operation.operation_id,
                scenario_name=scenario.name,
                ts_status=0,
                rust_status=0,
                passed=False,
                error=str(e)
            )
    
    def _send_request_obj(self, request: Any) -> dict[str, Any]:
        """Send a Request object to server"""
        parsed = urllib.parse.urlparse(request.url)
        
        conn = http.client.HTTPConnection(parsed.hostname, parsed.port or 80, timeout=self.timeout)
        
        # Build path with query string
        path = parsed.path
        if parsed.query:
            path += f"?{parsed.query}"
        
        filtered_headers = {}
        for name, value in request.headers.items():
            if name.lower() not in ("host", "connection"):
                filtered_headers[name] = value
        
        # Don't send body for GET/DELETE/HEAD — TS Azurite crashes on empty body
        send_body = request.body if request.method in ("PUT", "POST") else None
        conn.request(request.method, path, body=send_body, headers=filtered_headers)
        response = conn.getresponse()
        
        response_headers = {}
        for name, value in response.getheaders():
            response_headers[name] = value
        
        response_body = response.read()
        conn.close()
        
        return {
            "status_code": response.status,
            "reason": response.reason,
            "headers": response_headers,
            "body": response_body,
        }
    
    def _send_request(
        self,
        host: str,
        port: int,
        method: str,
        path: str,
        headers: dict[str, str],
        body: bytes
    ) -> dict[str, Any]:
        """Send request to a server"""
        conn = http.client.HTTPConnection(host, port, timeout=self.timeout)
        
        filtered_headers = {}
        for name, value in headers.items():
            if name.lower() not in ("host", "connection"):
                filtered_headers[name] = value
        
        send_body = body if method in ("PUT", "POST") else None
        conn.request(method, path, body=send_body, headers=filtered_headers)
        response = conn.getresponse()
        
        response_headers = {}
        for name, value in response.getheaders():
            response_headers[name] = value
        
        response_body = response.read()
        conn.close()
        
        return {
            "status_code": response.status,
            "reason": response.reason,
            "headers": response_headers,
            "body": response_body,
        }
    
    def run_tier(self, tier: int) -> list[ScenarioResult]:
        """Run all operations in a tier"""
        operations = [op for op in self.spec.get_operations() if op.tier == tier]
        results = []
        
        print(f"\n=== Running Tier {tier} ({len(operations)} operations) ===", flush=True)
        self.reset_state()  # Clean slate before tier
        
        for operation in operations:
            op_results = self.run_operation(operation.operation_id)
            results.extend(op_results)
        
        return results
    
    def run_group(self, group: str) -> list[ScenarioResult]:
        """Run all operations in a resource group"""
        operations = [op for op in self.spec.get_operations() if op.group.lower() == group.lower()]
        results = []
        
        print(f"\n=== Running Group '{group}' ({len(operations)} operations) ===", flush=True)
        self.reset_state()  # Clean slate before group
        
        for operation in operations:
            op_results = self.run_operation(operation.operation_id)
            results.extend(op_results)
        
        return results
    
    def run_all(self) -> list[ScenarioResult]:
        """Run all operations in dependency order"""
        all_operations = self.spec.get_operations()
        results = []
        
        print(f"\n=== Running All Operations ({len(all_operations)} total) ===", flush=True)
        self.reset_state()  # Clean slate before full run
        
        for operation in all_operations:
            op_results = self.run_operation(operation.operation_id)
            results.extend(op_results)
        
        return results
    
    def reset_state(self) -> None:
        """Delete all containers on both servers"""
        if self.verbose:
            print("  Resetting state (deleting all containers)...", flush=True)
        
        self._clear_containers(self.ts_host, self.ts_port, "TypeScript")
        self._clear_containers(self.rust_host, self.rust_port, "Rust")
    
    def _make_auth_request(self, host: str, port: int, method: str, path: str,
                           query_params: list = None, body: bytes = b"") -> tuple:
        """Send an authenticated request. Returns (status, headers, body)."""
        import datetime as dt
        query_params = query_params or []
        headers = {
            "x-ms-version": "2021-10-04",
            "x-ms-date": dt.datetime.now(dt.timezone.utc).strftime("%a, %d %b %Y %H:%M:%S GMT"),
        }
        # Only include Content-Length for methods that carry a body
        if method in ("PUT", "POST") and body:
            headers["Content-Length"] = str(len(body))
        
        builder = RequestBuilder(
            account_name="devstoreaccount1",
            account_key="Eby8vdM02xNOcqFlqUwJPLlmEtlCDXJ1OUzFT50uSRZ6IFsuFq2UVErCz4I6tq/K1SZFPTOtr/KBHBeksoGMGw==",
            host=host, port=port
        )
        auth = builder.compute_shared_key(method, path, headers, query_params, body)
        headers["Authorization"] = auth
        
        conn = http.client.HTTPConnection(host, port, timeout=self.timeout)
        qstring = "&".join(f"{k}={v}" for k, v in query_params) if query_params else ""
        full_path = path + ("?" + qstring if qstring else "")
        # Don't send body for GET/DELETE/HEAD — TS Azurite crashes on DELETE with body
        send_body = body if method in ("PUT", "POST") and body else None
        conn.request(method, full_path, body=send_body, headers=headers)
        resp = conn.getresponse()
        resp_body = resp.read()
        resp_headers = dict(resp.getheaders())
        conn.close()
        return (resp.status, resp_headers, resp_body)
    
    def _clear_containers(self, host: str, port: int, name: str) -> None:
        """Delete all blob containers on a server using authenticated requests"""
        try:
            acct = "devstoreaccount1"
            path = f"/{acct}"
            qp = [("comp", "list")]
            
            status, _, body = self._make_auth_request(host, port, "GET", path, qp)
            
            if status == 200:
                root = ET.fromstring(body)
                containers = []
                
                for container_elem in root.findall(".//Containers/Container/Name"):
                    if container_elem.text:
                        containers.append(container_elem.text)
                
                for container_name in containers:
                    try:
                        del_path = f"/{acct}/{container_name}"
                        del_qp = [("restype", "container")]
                        del_status, _, _ = self._make_auth_request(
                            host, port, "DELETE", del_path, del_qp
                        )
                        if self.verbose:
                            print(f"    Deleted '{container_name}' from {name} ({del_status})", flush=True)
                    except Exception as e:
                        if self.verbose:
                            print(f"    Failed to delete {container_name} from {name}: {e}", file=sys.stderr)
            elif self.verbose:
                print(f"    List containers from {name}: status {status}", file=sys.stderr)
            
        except Exception as e:
            if self.verbose:
                print(f"    Failed to clear containers from {name}: {e}", file=sys.stderr)


class ReportGenerator:
    """Generates coverage matrix from results"""
    
    def print_summary(self, results: list[ScenarioResult]) -> None:
        """Print console summary with coverage matrix"""
        if not results:
            print("\nNo results to report.", flush=True)
            return
        
        print("\n" + "=" * 80, flush=True)
        print("=== Differential Test Results ===", flush=True)
        print("=" * 80, flush=True)
        
        passed_count = sum(1 for r in results if r.passed)
        failed_count = sum(1 for r in results if not r.passed and not r.error)
        error_count = sum(1 for r in results if r.error)
        
        print(f"\nTotal: {len(results)} scenarios", flush=True)
        print(f"  ✓ Passed: {passed_count}", flush=True)
        print(f"  ✗ Failed: {failed_count}", flush=True)
        if error_count > 0:
            print(f"  ⚠ Errors: {error_count}", flush=True)
        
        if failed_count > 0:
            print("\n--- Failed Scenarios ---", flush=True)
            for result in results:
                if not result.passed and not result.error:
                    print(f"\n[FAIL] {result.operation_id} / {result.scenario_name}", flush=True)
                    print(f"  TS Status: {result.ts_status}, Rust Status: {result.rust_status}", flush=True)
                    for diff in result.differences[:5]:
                        print(f"  {diff}", flush=True)
                    if len(result.differences) > 5:
                        print(f"  ... ({len(result.differences) - 5} more differences)", flush=True)
        
        if error_count > 0:
            print("\n--- Errors ---", flush=True)
            for result in results:
                if result.error:
                    print(f"\n[ERROR] {result.operation_id} / {result.scenario_name}", flush=True)
                    print(f"  {result.error}", flush=True)
        
        print("\n" + "=" * 80, flush=True)
    
    def save_json(self, results: list[ScenarioResult], path: str) -> None:
        """Save detailed results as JSON for later analysis"""
        output = []
        for result in results:
            output.append({
                "operation_id": result.operation_id,
                "scenario_name": result.scenario_name,
                "ts_status": result.ts_status,
                "rust_status": result.rust_status,
                "passed": result.passed,
                "differences": result.differences,
                "setup_divergence": result.setup_divergence,
                "error": result.error,
            })
        
        with open(path, "w", encoding="utf-8") as f:
            json.dump(output, f, indent=2)
        
        print(f"\nResults saved to: {path}", flush=True)


class MockSwaggerSpec:
    """Mock swagger spec for testing (replace with Samwise's module)"""
    
    def get_operation(self, operation_id: str) -> Any:
        return {"id": operation_id, "method": "GET", "path": "/?comp=list"}
    
    def get_request_builder(self, operation_id: str) -> Any:
        return MockRequestBuilder()
    
    def get_operations_by_tier(self, tier: int) -> list[str]:
        return []
    
    def get_operations_by_group(self, group: str) -> list[str]:
        return []
    
    def get_all_operations(self) -> list[str]:
        return []


class MockRequestBuilder:
    """Mock request builder for testing (replace with Samwise's module)"""
    
    def build(self, overrides: dict[str, Any]) -> tuple[str, str, dict[str, str], bytes]:
        return ("GET", "/?comp=list", {}, b"")


def parse_args() -> argparse.Namespace:
    parser = argparse.ArgumentParser(
        description="Swagger-Based Differential Test Runner"
    )
    parser.add_argument(
        "--ts-host",
        default="127.0.0.1",
        help="TypeScript Azurite host (default: 127.0.0.1)"
    )
    parser.add_argument(
        "--ts-port",
        type=int,
        default=10001,
        help="TypeScript Azurite blob port (default: 10001)"
    )
    parser.add_argument(
        "--rust-host",
        default="127.0.0.1",
        help="Rust Azurite host (default: 127.0.0.1)"
    )
    parser.add_argument(
        "--rust-port",
        type=int,
        default=10000,
        help="Rust Azurite blob port (default: 10000)"
    )
    parser.add_argument(
        "--spec",
        type=pathlib.Path,
        default=pathlib.Path("swagger/blob-storage-2021-10-04.json"),
        help="Path to Swagger specification file"
    )
    parser.add_argument(
        "--operations",
        help="Comma-separated list of operation IDs to run"
    )
    parser.add_argument(
        "--tier",
        type=int,
        help="Run only operations in this dependency tier"
    )
    parser.add_argument(
        "--group",
        help="Run only operations in this resource group (Service, Container, Blob, etc.)"
    )
    parser.add_argument(
        "--output",
        help="JSON output file for detailed results"
    )
    parser.add_argument(
        "--verbose",
        "-v",
        action="store_true",
        help="Print detailed request/response information"
    )
    parser.add_argument(
        "--timeout",
        type=int,
        default=30,
        help="Request timeout in seconds (default: 30)"
    )
    return parser.parse_args()


def main() -> int:
    args = parse_args()
    
    print("=" * 80, flush=True)
    print("Swagger-Based Differential Test Runner", flush=True)
    print("=" * 80, flush=True)
    
    if not HAVE_PARSER:
        print("✗ swagger_parser module not available. Using mock spec.", file=sys.stderr)
        spec = MockSwaggerSpec()
    else:
        try:
            spec = SwaggerSpec(str(args.spec))
            print(f"✓ Loaded spec from: {args.spec}", flush=True)
        except Exception as e:
            print(f"✗ Failed to load spec from {args.spec}: {e}", file=sys.stderr)
            print("Using mock spec for testing.", file=sys.stderr)
            spec = MockSwaggerSpec()
    
    runner = DifferentialRunner(
        ts_host=args.ts_host,
        ts_port=args.ts_port,
        rust_host=args.rust_host,
        rust_port=args.rust_port,
        spec=spec,
        verbose=args.verbose,
        timeout=args.timeout
    )
    
    if not runner.check_servers():
        print("\n✗ Server connectivity check failed. Ensure both servers are running.", file=sys.stderr)
        return 1
    
    results = []
    
    if args.operations:
        operation_ids = [op.strip() for op in args.operations.split(",")]
        for operation_id in operation_ids:
            results.extend(runner.run_operation(operation_id))
    elif args.tier is not None:
        results = runner.run_tier(args.tier)
    elif args.group:
        results = runner.run_group(args.group)
    else:
        results = runner.run_all()
    
    report_gen = ReportGenerator()
    report_gen.print_summary(results)
    
    if args.output:
        report_gen.save_json(results, args.output)
    
    failed_count = sum(1 for r in results if not r.passed)
    return 0 if failed_count == 0 else 1


if __name__ == "__main__":
    sys.exit(main())
