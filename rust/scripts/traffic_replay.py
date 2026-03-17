#!/usr/bin/env python3
"""
Traffic Replay and Comparison Tool for Azurite Differential Testing

Replays recorded HTTP traffic against Rust Azurite and compares responses
to the recorded TypeScript responses using normalization from differential_test.py.

Usage:
    python3 traffic_replay.py --corpus-dir rust/scripts/traffic_corpus
    python3 traffic_replay.py --corpus-dir rust/scripts/traffic_corpus --services blob

Architecture:
    Replay Tool → Rust Azurite (:10000/:10001/:10002)
    Compare responses to recorded corpus
"""
import argparse
import base64
import http.client
import json
import pathlib
import re
import sys
import time
import xml.etree.ElementTree as ET
from dataclasses import dataclass
from typing import Any, Optional

# Import normalization logic from differential_test.py
# We'll inline the key functions to avoid dependencies

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

TARGET_PORTS = {
    "blob": 10000,
    "queue": 10001,
    "table": 10002,
}

TARGET_HOST = "127.0.0.1"


@dataclass
class ComparisonResult:
    """Result of comparing recorded vs actual response"""
    passed: bool
    sequence: int
    method: str
    path: str
    status: int
    details: list[str]


class TrafficReplayer:
    """Replays recorded traffic and compares responses"""
    
    def __init__(self, corpus_dir: pathlib.Path, target_host: str = TARGET_HOST):
        self.corpus_dir = corpus_dir
        self.target_host = target_host
    
    def replay_service(self, service: str) -> list[ComparisonResult]:
        """Replay all exchanges for a service"""
        corpus_file = self.corpus_dir / f"{service}_traffic.json"
        
        if not corpus_file.exists():
            print(f"[{service.upper()}] Corpus file not found: {corpus_file}", file=sys.stderr)
            return []
        
        with corpus_file.open("r", encoding="utf-8") as f:
            corpus_data = json.load(f)
        
        exchanges = corpus_data.get("exchanges", [])
        print(f"\n=== Traffic Replay: {service} ({len(exchanges)} exchanges) ===", flush=True)
        
        results = []
        for exchange in exchanges:
            result = self._replay_exchange(service, exchange)
            results.append(result)
            
            status_symbol = "✓" if result.passed else "✗"
            status_word = "PASS" if result.passed else "FAIL"
            print(f"[{status_word}] #{result.sequence:04d} {result.method} {result.path} → {result.status}", flush=True)
            
            if not result.passed:
                for detail in result.details[:5]:
                    print(f"  {detail}", flush=True)
                if len(result.details) > 5:
                    print(f"  ... ({len(result.details) - 5} more differences)", flush=True)
        
        return results
    
    def _replay_exchange(self, service: str, exchange: dict[str, Any]) -> ComparisonResult:
        """Replay a single exchange and compare"""
        seq = exchange["sequence_number"]
        request = exchange["request"]
        recorded_response = exchange["response"]
        
        method = request["method"]
        path = request["path"]
        headers = request["headers"]
        body_data = request["body"]
        
        body = self._decode_body(body_data)
        
        try:
            actual_response = self._send_request(service, method, path, headers, body)
            
            result = self._compare_responses(
                seq,
                method,
                path,
                recorded_response,
                actual_response
            )
            
            return result
            
        except Exception as e:
            return ComparisonResult(
                passed=False,
                sequence=seq,
                method=method,
                path=path,
                status=0,
                details=[f"Request failed: {e}"]
            )
    
    def _decode_body(self, body_data: dict[str, Any]) -> bytes:
        """Decode body from corpus format"""
        encoding = body_data.get("encoding", "empty")
        data = body_data.get("data", "")
        
        if encoding == "empty":
            return b""
        elif encoding == "utf-8":
            return data.encode("utf-8")
        elif encoding == "base64":
            return base64.b64decode(data)
        else:
            raise ValueError(f"Unknown body encoding: {encoding}")
    
    def _send_request(
        self,
        service: str,
        method: str,
        path: str,
        headers: dict[str, str],
        body: bytes
    ) -> dict[str, Any]:
        """Send request to target server"""
        port = TARGET_PORTS[service]
        conn = http.client.HTTPConnection(self.target_host, port, timeout=120)
        
        filtered_headers = {}
        for name, value in headers.items():
            if name.lower() not in ("host", "connection"):
                filtered_headers[name] = value
        
        conn.request(method, path, body=body, headers=filtered_headers)
        response = conn.getresponse()
        
        response_headers = {}
        for name, value in response.getheaders():
            response_headers[name] = value
        
        response_body = response.read()
        
        return {
            "status_code": response.status,
            "reason": response.reason,
            "headers": response_headers,
            "body": response_body,
        }
    
    def _compare_responses(
        self,
        seq: int,
        method: str,
        path: str,
        recorded: dict[str, Any],
        actual: dict[str, Any]
    ) -> ComparisonResult:
        """Compare recorded and actual responses"""
        details = []
        
        recorded_status = recorded["status_code"]
        actual_status = actual["status_code"]
        
        if recorded_status != actual_status:
            details.append(f"Status code: {recorded_status} != {actual_status}")
        
        header_diff = self._compare_headers(recorded["headers"], actual["headers"])
        if header_diff:
            details.append("Headers differ:")
            details.extend(header_diff)
        
        body_diff = self._compare_bodies(
            self._decode_body(recorded["body"]) if isinstance(recorded["body"], dict) else recorded["body"].encode("utf-8") if isinstance(recorded["body"], str) else b"",
            actual["body"],
            recorded["headers"],
            actual["headers"]
        )
        if body_diff:
            details.append("Body differs:")
            details.extend(body_diff)
        
        return ComparisonResult(
            passed=len(details) == 0,
            sequence=seq,
            method=method,
            path=path,
            status=actual_status,
            details=details
        )
    
    def _compare_headers(
        self,
        recorded: dict[str, str],
        actual: dict[str, str]
    ) -> list[str]:
        """Compare headers with normalization"""
        recorded_norm = self._normalize_headers(recorded)
        actual_norm = self._normalize_headers(actual)
        
        differences = []
        
        recorded_keys = set(recorded_norm.keys())
        actual_keys = set(actual_norm.keys())
        
        missing_in_actual = recorded_keys - actual_keys
        extra_in_actual = actual_keys - recorded_keys
        
        if missing_in_actual:
            differences.append(f"  Missing in actual: {sorted(missing_in_actual)}")
        if extra_in_actual:
            differences.append(f"  Extra in actual: {sorted(extra_in_actual)}")
        
        for key in sorted(recorded_keys & actual_keys):
            if recorded_norm[key] != actual_norm[key]:
                differences.append(f"  {key}: {recorded_norm[key]} != {actual_norm[key]}")
        
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
        recorded: bytes,
        actual: bytes,
        recorded_headers: dict[str, str],
        actual_headers: dict[str, str]
    ) -> list[str]:
        """Compare bodies with content-type aware normalization"""
        if not recorded and not actual:
            return []
        
        content_type = self._detect_content_type(recorded_headers, actual_headers, recorded, actual)
        
        if content_type == "json":
            return self._compare_json_bodies(recorded, actual)
        elif content_type == "xml":
            return self._compare_xml_bodies(recorded, actual)
        else:
            return self._compare_binary_bodies(recorded, actual)
    
    def _detect_content_type(
        self,
        recorded_headers: dict[str, str],
        actual_headers: dict[str, str],
        recorded_body: bytes,
        actual_body: bytes
    ) -> str:
        """Detect content type for body comparison"""
        content_types = []
        
        for headers in [recorded_headers, actual_headers]:
            ct = headers.get("Content-Type", headers.get("content-type", "")).lower()
            content_types.append(ct)
        
        combined = " ".join(content_types)
        
        if "json" in combined:
            return "json"
        if "xml" in combined:
            return "xml"
        
        sample = next((body.lstrip() for body in [recorded_body, actual_body] if body.strip()), b"")
        if sample.startswith((b"{", b"[")):
            return "json"
        if sample.startswith(b"<"):
            return "xml"
        
        return "binary"
    
    def _compare_json_bodies(self, recorded: bytes, actual: bytes) -> list[str]:
        """Compare JSON bodies with normalization"""
        try:
            recorded_obj = json.loads(recorded.decode("utf-8"))
            actual_obj = json.loads(actual.decode("utf-8"))
        except Exception as e:
            return [f"JSON parse failed: {e}"]
        
        recorded_norm = self._normalize_json(recorded_obj)
        actual_norm = self._normalize_json(actual_obj)
        
        if recorded_norm == actual_norm:
            return []
        
        return [
            "  JSON structures differ",
            f"  Recorded: {json.dumps(recorded_norm, indent=2)[:500]}",
            f"  Actual: {json.dumps(actual_norm, indent=2)[:500]}",
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
    
    def _compare_xml_bodies(self, recorded: bytes, actual: bytes) -> list[str]:
        """Compare XML bodies with normalization"""
        try:
            recorded_tree = ET.fromstring(recorded)
            actual_tree = ET.fromstring(actual)
        except Exception as e:
            return [f"XML parse failed: {e}"]
        
        recorded_norm = self._normalize_xml(recorded_tree)
        actual_norm = self._normalize_xml(actual_tree)
        
        if recorded_norm == actual_norm:
            return []
        
        return [
            "  XML structures differ",
            f"  Recorded: {json.dumps(recorded_norm, indent=2)[:500]}",
            f"  Actual: {json.dumps(actual_norm, indent=2)[:500]}",
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
    
    def _compare_binary_bodies(self, recorded: bytes, actual: bytes) -> list[str]:
        """Compare binary bodies"""
        if recorded == actual:
            return []
        
        return [
            f"  Binary bodies differ: {len(recorded)} bytes vs {len(actual)} bytes"
        ]


def print_summary(service_results: dict[str, list[ComparisonResult]]) -> int:
    """Print summary of all results"""
    print("\n" + "=" * 60, flush=True)
    print("=== Summary ===", flush=True)
    print("=" * 60, flush=True)
    
    total_passed = 0
    total_failed = 0
    
    for service, results in service_results.items():
        passed = sum(1 for r in results if r.passed)
        failed = sum(1 for r in results if not r.passed)
        total_passed += passed
        total_failed += failed
        
        print(f"{service}: {passed}/{len(results)} passed ({failed} failures)", flush=True)
    
    print(f"\nTotal: {total_passed}/{total_passed + total_failed} passed ({total_failed} failures)", flush=True)
    
    return 0 if total_failed == 0 else 1


def parse_args() -> argparse.Namespace:
    parser = argparse.ArgumentParser(
        description="Replay recorded traffic and compare responses"
    )
    parser.add_argument(
        "--corpus-dir",
        type=pathlib.Path,
        default=pathlib.Path(__file__).parent / "traffic_corpus",
        help="Directory containing corpus files",
    )
    parser.add_argument(
        "--services",
        nargs="+",
        choices=["blob", "queue", "table"],
        default=["blob", "queue", "table"],
        help="Services to replay (default: all)",
    )
    parser.add_argument(
        "--target-host",
        default=TARGET_HOST,
        help=f"Target host (default: {TARGET_HOST})",
    )
    return parser.parse_args()


def main() -> int:
    args = parse_args()
    
    replayer = TrafficReplayer(args.corpus_dir, args.target_host)
    
    service_results = {}
    for service in args.services:
        results = replayer.replay_service(service)
        service_results[service] = results
    
    return print_summary(service_results)


if __name__ == "__main__":
    sys.exit(main())
