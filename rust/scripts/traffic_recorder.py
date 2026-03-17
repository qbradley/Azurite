#!/usr/bin/env python3
"""
Traffic Recording Proxy for Azurite Differential Testing

Records HTTP traffic from TypeScript integration tests to create a corpus
for replay-based differential testing against Rust Azurite.

Usage:
    python3 traffic_recorder.py --output-dir rust/scripts/traffic_corpus

Architecture:
    Tests → Proxy (:11000/:11001/:11002) → TS Azurite (:10000/:10001/:10002)
    
The proxy records each request/response pair as JSON with normalization
for replay against Rust implementation.
"""
import argparse
import base64
import http.client
import json
import pathlib
import socket
import sys
import threading
import time
from dataclasses import dataclass, field
from http.server import BaseHTTPRequestHandler, HTTPServer
from typing import Any, Optional


SERVICE_PORTS = {
    "blob": {"proxy": 11000, "backend": 10000},
    "queue": {"proxy": 11001, "backend": 10001},
    "table": {"proxy": 11002, "backend": 10002},
}

BACKEND_HOST = "127.0.0.1"


@dataclass
class ExchangeRecord:
    """Recorded HTTP request/response pair"""
    sequence_number: int
    timestamp: float
    service: str
    request: dict[str, Any]
    response: dict[str, Any]


class RecordingState:
    """Thread-safe recording state"""
    def __init__(self, service: str):
        self.service = service
        self.exchanges: list[ExchangeRecord] = []
        self.lock = threading.Lock()
        self.sequence = 0
        
    def add_exchange(self, request: dict[str, Any], response: dict[str, Any]) -> int:
        with self.lock:
            self.sequence += 1
            seq = self.sequence
            exchange = ExchangeRecord(
                sequence_number=seq,
                timestamp=time.time(),
                service=self.service,
                request=request,
                response=response,
            )
            self.exchanges.append(exchange)
            return seq
    
    def get_exchanges(self) -> list[ExchangeRecord]:
        with self.lock:
            return list(self.exchanges)


class RecordingProxyHandler(BaseHTTPRequestHandler):
    """HTTP proxy handler that records and forwards requests"""
    
    recording_state: RecordingState
    backend_port: int
    
    def do_GET(self) -> None:
        self._handle_request()
    
    def do_POST(self) -> None:
        self._handle_request()
    
    def do_PUT(self) -> None:
        self._handle_request()
    
    def do_DELETE(self) -> None:
        self._handle_request()
    
    def do_HEAD(self) -> None:
        self._handle_request()
    
    def do_OPTIONS(self) -> None:
        self._handle_request()
    
    def do_PATCH(self) -> None:
        self._handle_request()
    
    def _handle_request(self) -> None:
        """Forward request to backend and record the exchange"""
        try:
            request_body = self._read_request_body()
            request_headers = self._capture_request_headers()
            
            backend_response = self._forward_to_backend(
                self.command,
                self.path,
                request_headers,
                request_body
            )
            
            response_headers = self._capture_response_headers(backend_response)
            response_body = backend_response.read()
            
            seq = self._record_exchange(
                request_headers,
                request_body,
                backend_response.status,
                backend_response.reason,
                response_headers,
                response_body
            )
            
            print(f"[REC] {seq:04d} {self.command} {self.path} → {backend_response.status}", flush=True)
            
            self._send_response_to_client(
                backend_response.status,
                backend_response.reason,
                response_headers,
                response_body
            )
            
        except Exception as e:
            print(f"[ERR] Proxy error: {e}", file=sys.stderr, flush=True)
            self.send_error(500, f"Proxy error: {e}")
    
    def _read_request_body(self) -> bytes:
        """Read request body from client"""
        content_length = self.headers.get("Content-Length")
        if content_length:
            return self.rfile.read(int(content_length))
        return b""
    
    def _capture_request_headers(self) -> list[tuple[str, str]]:
        """Capture request headers as list of tuples"""
        headers = []
        for name, value in self.headers.items():
            headers.append((name, value))
        return headers
    
    def _forward_to_backend(
        self,
        method: str,
        path: str,
        headers: list[tuple[str, str]],
        body: bytes
    ) -> http.client.HTTPResponse:
        """Forward request to backend Azurite server"""
        conn = http.client.HTTPConnection(BACKEND_HOST, self.backend_port, timeout=120)
        
        headers_dict = {}
        for name, value in headers:
            if name.lower() not in ("host", "connection"):
                headers_dict[name] = value
        
        conn.request(method, path, body=body, headers=headers_dict)
        return conn.getresponse()
    
    def _capture_response_headers(self, response: http.client.HTTPResponse) -> list[tuple[str, str]]:
        """Capture response headers as list of tuples"""
        headers = []
        for name, value in response.getheaders():
            headers.append((name, value))
        return headers
    
    def _record_exchange(
        self,
        request_headers: list[tuple[str, str]],
        request_body: bytes,
        status_code: int,
        reason: str,
        response_headers: list[tuple[str, str]],
        response_body: bytes
    ) -> int:
        """Record the request/response exchange"""
        request_record = {
            "method": self.command,
            "path": self.path,
            "headers": {name: value for name, value in request_headers},
            "body": self._encode_body(request_body, dict(request_headers)),
        }
        
        response_record = {
            "status_code": status_code,
            "reason": reason,
            "headers": {name: value for name, value in response_headers},
            "body": self._encode_body(response_body, dict(response_headers)),
        }
        
        return self.recording_state.add_exchange(request_record, response_record)
    
    def _encode_body(self, body: bytes, headers: dict[str, str]) -> dict[str, Any]:
        """Encode body for JSON storage"""
        if not body:
            return {"encoding": "empty", "data": ""}
        
        content_type = headers.get("Content-Type", "").lower()
        
        if "text" in content_type or "xml" in content_type or "json" in content_type:
            try:
                text = body.decode("utf-8")
                return {"encoding": "utf-8", "data": text}
            except UnicodeDecodeError:
                pass
        
        return {"encoding": "base64", "data": base64.b64encode(body).decode("ascii")}
    
    def _send_response_to_client(
        self,
        status: int,
        reason: str,
        headers: list[tuple[str, str]],
        body: bytes
    ) -> None:
        """Send backend response to client"""
        # Use send_response_only to avoid auto-added Server/Date headers
        self.send_response_only(status, reason)
        
        for name, value in headers:
            if name.lower() not in ("connection", "transfer-encoding", "content-length"):
                self.send_header(name, value)
        
        self.send_header("Content-Length", str(len(body)))
        self.end_headers()
        
        if body:
            self.wfile.write(body)
    
    def log_message(self, format: str, *args: object) -> None:
        """Suppress default logging"""
        pass


class RecordingProxy:
    """Multi-threaded recording proxy for one service"""
    
    def __init__(self, service: str, output_dir: pathlib.Path):
        self.service = service
        self.output_dir = output_dir
        self.recording_state = RecordingState(service)
        
        port_config = SERVICE_PORTS[service]
        self.proxy_port = port_config["proxy"]
        self.backend_port = port_config["backend"]
        
        handler_class = type(
            f"RecordingProxyHandler_{service}",
            (RecordingProxyHandler,),
            {
                "recording_state": self.recording_state,
                "backend_port": self.backend_port,
            }
        )
        
        self.server = HTTPServer(("127.0.0.1", self.proxy_port), handler_class)
        self.thread: Optional[threading.Thread] = None
    
    def start(self) -> None:
        """Start proxy server in background thread"""
        self.thread = threading.Thread(target=self.server.serve_forever, daemon=True)
        self.thread.start()
        
        self._wait_for_ready()
        print(f"[{self.service.upper()}] Recording proxy listening on ::{self.proxy_port} → ::{self.backend_port}", flush=True)
    
    def _wait_for_ready(self, timeout: float = 5.0) -> None:
        """Wait for proxy to be ready to accept connections"""
        deadline = time.time() + timeout
        while time.time() < deadline:
            try:
                sock = socket.socket(socket.AF_INET, socket.SOCK_STREAM)
                sock.settimeout(1.0)
                sock.connect(("127.0.0.1", self.proxy_port))
                sock.close()
                return
            except (ConnectionRefusedError, socket.timeout):
                time.sleep(0.1)
        raise RuntimeError(f"Proxy for {self.service} failed to start")
    
    def stop(self) -> None:
        """Stop proxy server"""
        self.server.shutdown()
        self.server.server_close()
        if self.thread:
            self.thread.join(timeout=5)
    
    def save_corpus(self) -> pathlib.Path:
        """Save recorded exchanges to JSON file"""
        exchanges = self.recording_state.get_exchanges()
        
        corpus_data = {
            "service": self.service,
            "recorded_at": time.time(),
            "exchange_count": len(exchanges),
            "exchanges": [
                {
                    "sequence_number": ex.sequence_number,
                    "timestamp": ex.timestamp,
                    "service": ex.service,
                    "request": ex.request,
                    "response": ex.response,
                }
                for ex in exchanges
            ]
        }
        
        self.output_dir.mkdir(parents=True, exist_ok=True)
        output_file = self.output_dir / f"{self.service}_traffic.json"
        
        with output_file.open("w", encoding="utf-8") as f:
            json.dump(corpus_data, f, indent=2, ensure_ascii=False)
        
        print(f"[{self.service.upper()}] Saved {len(exchanges)} exchanges to {output_file}", flush=True)
        return output_file
    
    def get_stats(self) -> dict[str, Any]:
        """Get recording statistics"""
        exchanges = self.recording_state.get_exchanges()
        return {
            "service": self.service,
            "exchange_count": len(exchanges),
            "sequence_range": (exchanges[0].sequence_number, exchanges[-1].sequence_number) if exchanges else (0, 0),
        }


def parse_args() -> argparse.Namespace:
    parser = argparse.ArgumentParser(
        description="Recording proxy for Azurite traffic capture"
    )
    parser.add_argument(
        "--output-dir",
        type=pathlib.Path,
        default=pathlib.Path(__file__).parent / "traffic_corpus",
        help="Directory to write corpus files (default: ./traffic_corpus)",
    )
    parser.add_argument(
        "--services",
        nargs="+",
        choices=["blob", "queue", "table"],
        default=["blob", "queue", "table"],
        help="Services to record (default: all)",
    )
    return parser.parse_args()


def main() -> int:
    args = parse_args()
    
    proxies: list[RecordingProxy] = []
    shutdown_requested = False

    def _shutdown(signum: int, frame: object) -> None:
        nonlocal shutdown_requested
        if not shutdown_requested:
            shutdown_requested = True
            print(f"\n=== Signal {signum} received, saving corpus ===", flush=True)
            _save_all()
            sys.exit(0)

    def _save_all() -> None:
        for p in proxies:
            p.stop()
        total = 0
        for p in proxies:
            stats = p.get_stats()
            print(f"{stats['service']}: {stats['exchange_count']} exchanges", flush=True)
            total += stats['exchange_count']
            p.save_corpus()
        print(f"\nTotal: {total} exchanges recorded", flush=True)
        print(f"Corpus directory: {args.output_dir}", flush=True)

    import signal
    signal.signal(signal.SIGTERM, _shutdown)
    signal.signal(signal.SIGINT, _shutdown)

    try:
        for service in args.services:
            proxy = RecordingProxy(service, args.output_dir)
            proxy.start()
            proxies.append(proxy)
        
        print("\n=== Recording Proxy Ready ===", flush=True)
        print("Run your tests now. Press Ctrl+C when done.", flush=True)
        print("", flush=True)
        
        while True:
            time.sleep(1)
    
    except KeyboardInterrupt:
        print("\n\n=== Stopping Recording ===", flush=True)
    
    finally:
        if not shutdown_requested:
            _save_all()
    
    return 0


if __name__ == "__main__":
    sys.exit(main())
