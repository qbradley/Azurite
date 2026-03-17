"""
Swagger parser for Azure Blob Storage API 2021-10-04.

Parses the swagger spec, resolves $ref references, builds operation dependency DAG,
generates request templates, and computes SharedKey authentication.
"""

import base64
import datetime as dt
import hashlib
import hmac
import json
import random
import string
import urllib.parse
import uuid
from dataclasses import dataclass, field
from typing import Any, Optional


@dataclass
class Parameter:
    """Resolved parameter (no more $refs)"""
    name: str
    location: str  # header, query, path, body
    required: bool
    type: str
    enum: Optional[list[str]] = None
    description: str = ""
    minimum: Optional[int] = None
    format: Optional[str] = None
    
    @classmethod
    def from_spec(cls, param_spec: dict[str, Any]) -> "Parameter":
        """Create Parameter from resolved swagger parameter spec"""
        return cls(
            name=param_spec["name"],
            location=param_spec["in"],
            required=param_spec.get("required", False),
            type=param_spec.get("type", "string"),
            enum=param_spec.get("enum"),
            description=param_spec.get("description", ""),
            minimum=param_spec.get("minimum"),
            format=param_spec.get("format"),
        )


@dataclass
class ResponseDef:
    """Response definition"""
    status_code: str
    description: str
    headers: dict[str, Any] = field(default_factory=dict)
    schema: Optional[dict[str, Any]] = None


@dataclass
class RequestBody:
    """Request body definition"""
    required: bool
    schema: Optional[dict[str, Any]] = None
    content_type: str = "application/octet-stream"


@dataclass
class Operation:
    """A single API operation with resolved parameters"""
    operation_id: str
    method: str  # GET, PUT, POST, DELETE, HEAD
    path_template: str  # /{containerName}/{blob}?comp=block
    tier: int  # dependency tier (0-4)
    group: str  # Service, Container, Blob, etc.
    parameters: list[Parameter]
    request_body: Optional[RequestBody] = None
    responses: dict[str, ResponseDef] = field(default_factory=dict)
    description: str = ""
    
    def get_parameter(self, name: str) -> Optional[Parameter]:
        """Get parameter by name"""
        for param in self.parameters:
            if param.name == name:
                return param
        return None


@dataclass
class SetupStep:
    """A step in the setup chain for an operation"""
    operation_id: str
    reason: str  # Why this step is needed


class SwaggerSpec:
    """Parsed and resolved swagger specification"""
    
    def __init__(self, spec_path: str):
        with open(spec_path) as f:
            self.raw_spec = json.load(f)
        
        self.parameters = self.raw_spec.get("parameters", {})
        self.definitions = self.raw_spec.get("definitions", {})
        self.paths = self.raw_spec.get("x-ms-paths", {})
        
        self._operations: dict[str, Operation] = {}
        self._parse_operations()
    
    def _resolve_ref(self, ref: str) -> dict[str, Any]:
        """Resolve a $ref reference"""
        if not ref.startswith("#/"):
            raise ValueError(f"Unsupported ref format: {ref}")
        
        parts = ref[2:].split("/")
        obj = self.raw_spec
        for part in parts:
            obj = obj[part]
        return obj
    
    def _resolve_parameter(self, param_spec: dict[str, Any]) -> dict[str, Any]:
        """Resolve parameter, handling $ref if present"""
        if "$ref" in param_spec:
            return self._resolve_ref(param_spec["$ref"])
        return param_spec
    
    def _parse_operations(self):
        """Parse all operations from x-ms-paths"""
        for path_template, path_obj in self.paths.items():
            # Path-level parameters (shared across operations)
            path_params = [
                self._resolve_parameter(p) for p in path_obj.get("parameters", [])
            ]
            
            for method, op_spec in path_obj.items():
                if method == "parameters":
                    continue
                
                operation_id = op_spec.get("operationId")
                if not operation_id:
                    continue
                
                # Combine path-level and operation-level parameters
                all_params = path_params + [
                    self._resolve_parameter(p) for p in op_spec.get("parameters", [])
                ]
                
                parameters = [Parameter.from_spec(p) for p in all_params]
                
                # Determine operation group
                tags = op_spec.get("tags", [])
                group = tags[0].capitalize() if tags else "Unknown"
                
                # Parse request body if present
                request_body = None
                for param in all_params:
                    if param.get("in") == "body":
                        request_body = RequestBody(
                            required=param.get("required", False),
                            schema=param.get("schema")
                        )
                        break
                
                # Parse responses
                responses = {}
                for status, resp_spec in op_spec.get("responses", {}).items():
                    responses[status] = ResponseDef(
                        status_code=status,
                        description=resp_spec.get("description", ""),
                        headers=resp_spec.get("headers", {}),
                        schema=resp_spec.get("schema")
                    )
                
                operation = Operation(
                    operation_id=operation_id,
                    method=method.upper(),
                    path_template=path_template,
                    tier=self._compute_tier(operation_id),
                    group=group,
                    parameters=parameters,
                    request_body=request_body,
                    responses=responses,
                    description=op_spec.get("description", "")
                )
                
                self._operations[operation_id] = operation
    
    def _compute_tier(self, operation_id: str) -> int:
        """Compute dependency tier for an operation"""
        # TIER 0 - No prerequisites
        tier_0 = {
            "Service_SetProperties", "Service_GetProperties", "Service_GetStatistics",
            "Service_ListContainersSegment", "Service_GetAccountInfo", 
            "Service_GetAccountInfoWithHead", "Service_FilterBlobs", 
            "Service_GetUserDelegationKey", "Service_SubmitBatch", "Container_Create"
        }
        
        # TIER 1 - Needs Container_Create
        tier_1 = {
            "Container_GetProperties", "Container_GetPropertiesWithHead", "Container_Delete",
            "Container_SetMetadata", "Container_GetAccessPolicy", "Container_SetAccessPolicy",
            "Container_Restore", "Container_SubmitBatch", "Container_FilterBlobs",
            "Container_AcquireLease", "Container_ListBlobFlatSegment", 
            "Container_ListBlobHierarchySegment", "Container_Rename", "Container_GetAccountInfo",
            "Container_GetAccountInfoWithHead", "BlockBlob_Upload", "PageBlob_Create", 
            "AppendBlob_Create"
        }
        
        # TIER 3 - Needs a lease
        tier_3_blob_lease = {
            "Blob_ReleaseLease", "Blob_RenewLease", "Blob_ChangeLease", "Blob_BreakLease"
        }
        tier_3_container_lease = {
            "Container_ReleaseLease", "Container_RenewLease", "Container_ChangeLease", 
            "Container_BreakLease"
        }
        
        # TIER 4 - Needs a snapshot
        tier_4 = {"Blob_CreateSnapshot"}
        
        if operation_id in tier_0:
            return 0
        elif operation_id in tier_1:
            return 1
        elif operation_id in tier_3_blob_lease or operation_id in tier_3_container_lease:
            return 3
        elif operation_id in tier_4:
            return 4
        else:
            # Everything else is TIER 2 (needs a blob)
            return 2
    
    def get_operations(self) -> list[Operation]:
        """All operations sorted by dependency tier"""
        return sorted(self._operations.values(), key=lambda op: (op.tier, op.operation_id))
    
    def get_operation(self, operation_id: str) -> Optional[Operation]:
        """Single operation by ID"""
        return self._operations.get(operation_id)
    
    def get_dependency_chain(self, operation_id: str) -> list[Operation]:
        """Operations that must run before this one (in order)"""
        dag = DependencyDAG()
        setup_steps = dag.get_setup_chain(operation_id)
        return [self._operations[step.operation_id] for step in setup_steps]


class DependencyDAG:
    """Operation dependency graph"""
    
    def get_setup_chain(self, operation_id: str) -> list[SetupStep]:
        """Return ordered list of operations needed before this one"""
        tier = self.get_tier(operation_id)
        steps: list[SetupStep] = []
        
        if tier >= 1:
            # Need a container
            steps.append(SetupStep("Container_Create", "Create container"))
        
        if tier >= 2:
            # Need a blob (choose BlockBlob for simplicity)
            steps.append(SetupStep("BlockBlob_Upload", "Create blob"))
        
        if tier == 3:
            # Need a lease
            if operation_id.startswith("Container_"):
                steps.append(SetupStep("Container_AcquireLease", "Acquire container lease"))
            else:
                steps.append(SetupStep("Blob_AcquireLease", "Acquire blob lease"))
        
        if tier == 4:
            # Need a snapshot
            steps.append(SetupStep("Blob_CreateSnapshot", "Create snapshot"))
        
        return steps
    
    def get_tier(self, operation_id: str) -> int:
        """Which tier (0-4) this operation is in"""
        # Reuse the same logic from SwaggerSpec
        spec = SwaggerSpec.__new__(SwaggerSpec)
        return spec._compute_tier(operation_id)


class RequestBuilder:
    """Builds HTTP requests from operations"""
    
    def __init__(self, account_name: str, account_key: str, host: str, port: int):
        self.account_name = account_name
        self.account_key = account_key
        self.host = host
        self.port = port
        self.base_url = f"http://{host}:{port}"
    
    def build_request(
        self, operation: Operation, overrides: Optional[dict[str, Any]] = None
    ) -> "Request":
        """
        Build a complete HTTP request with auth for this operation.
        overrides: parameter name -> value (for specific test scenarios)
        """
        overrides = overrides or {}
        
        # Generate parameter values
        param_values = self._generate_param_values(operation, overrides)
        
        # Build path
        path = self._build_path(operation.path_template, param_values)
        
        # Build query params
        query_params = self._build_query_params(operation, param_values)
        
        # Build headers
        headers = self._build_headers(operation, param_values)
        
        # Build body (may modify headers, e.g., Content-Type for batch)
        body = self._build_body(operation, param_values, headers, overrides)
        
        # Update headers based on body and method
        self._update_headers_for_body(headers, body, operation.method)
        
        # Compute authorization
        auth = self.compute_shared_key(operation.method, path, headers, query_params, body)
        headers["Authorization"] = auth
        
        # Build full URL
        query_string = "&".join(f"{k}={urllib.parse.quote(str(v))}" for k, v in query_params)
        url = f"{self.base_url}{path}"
        if query_string:
            url += f"?{query_string}"
        
        return Request(
            method=operation.method,
            url=url,
            headers=headers,
            body=body,
            path=path,
            query_params=query_params
        )
    
    def _generate_param_values(
        self, operation: Operation, overrides: dict[str, Any]
    ) -> dict[str, Any]:
        """Generate default parameter values"""
        values = {}
        
        for param in operation.parameters:
            if param.name in overrides:
                values[param.name] = overrides[param.name]
                continue
            
            # Generate defaults based on parameter name and type
            if param.name == "url":
                values[param.name] = self.base_url
            elif param.name == "containerName":
                values[param.name] = overrides.get("_containerName", 
                    f"testcontainer{self._random_suffix()}")
            elif param.name == "blob":
                values[param.name] = overrides.get("_blob", f"testblob{self._random_suffix()}")
            elif param.name == "x-ms-version":
                values[param.name] = "2021-10-04"
            elif param.name == "x-ms-date":
                values[param.name] = self._format_rfc1123(dt.datetime.now(dt.timezone.utc))
            elif param.name == "timeout":
                values[param.name] = "30"
            elif param.name == "x-ms-blob-type":
                # Infer from operation
                if "BlockBlob" in operation.operation_id:
                    values[param.name] = "BlockBlob"
                elif "PageBlob" in operation.operation_id:
                    values[param.name] = "PageBlob"
                elif "AppendBlob" in operation.operation_id:
                    values[param.name] = "AppendBlob"
            elif param.name == "Content-Length":
                # Will be computed from body
                continue
            elif param.name == "Content-Type":
                if operation.request_body:
                    # Batch operations need special Content-Type
                    if "SubmitBatch" in operation.operation_id:
                        # Will be set in _build_body with batch boundary
                        pass
                    else:
                        values[param.name] = "application/octet-stream"
            elif param.name == "x-ms-lease-duration":
                values[param.name] = "15"
            elif param.name == "x-ms-blob-content-length" and "PageBlob" in operation.operation_id:
                values[param.name] = "512"
            elif param.required:
                # Required parameters need defaults
                if param.enum:
                    values[param.name] = param.enum[0]
                elif param.type == "integer":
                    values[param.name] = str(param.minimum) if param.minimum else "0"
                elif param.type == "boolean":
                    values[param.name] = "false"
                else:
                    values[param.name] = f"default_{param.name}"
            # Skip optional parameters by default unless explicitly set in overrides
        
        return values
    
    def _build_path(self, template: str, param_values: dict[str, Any]) -> str:
        """Build path from template, substituting path parameters"""
        path = template.split("?")[0]  # Remove query part
        
        # Replace path parameters
        path = path.replace("{containerName}", param_values.get("containerName", "container"))
        path = path.replace("{blob}", param_values.get("blob", "blob"))
        
        # Prepend account name
        return f"/{self.account_name}{path}"
    
    def _build_query_params(
        self, operation: Operation, param_values: dict[str, Any]
    ) -> list[tuple[str, str]]:
        """Build query parameters"""
        params: list[tuple[str, str]] = []
        seen_keys = set()
        
        # Parse query from path template (excluding x-ms-paths discriminators)
        # Discriminators: BlockBlob, PageBlob, AppendBlob (without values)
        discriminators = {"blockblob", "pageblob", "appendblob"}
        
        if "?" in operation.path_template:
            query_part = operation.path_template.split("?", 1)[1]
            for pair in query_part.split("&"):
                if "=" in pair:
                    key, value = pair.split("=", 1)
                    params.append((key, value))
                    seen_keys.add(key.lower())
                else:
                    # Standalone query param (no value)
                    # Skip if it's a blob type discriminator
                    if pair.lower() not in discriminators:
                        params.append((pair, ""))
                        seen_keys.add(pair.lower())
        
        # Add query parameters from operation (avoid duplicates)
        for param in operation.parameters:
            if param.location == "query" and param.name in param_values:
                if param.name.lower() not in seen_keys:
                    params.append((param.name, str(param_values[param.name])))
        
        return params
    
    def _build_headers(
        self, operation: Operation, param_values: dict[str, Any]
    ) -> dict[str, str]:
        """Build request headers"""
        headers = {}
        
        for param in operation.parameters:
            if param.location == "header" and param.name in param_values:
                headers[param.name] = str(param_values[param.name])
        
        # Ensure required headers
        if "x-ms-version" not in headers:
            headers["x-ms-version"] = "2021-10-04"
        if "x-ms-date" not in headers:
            headers["x-ms-date"] = self._format_rfc1123(dt.datetime.now(dt.timezone.utc))
        
        return headers
    
    def _build_body(self, operation: Operation, param_values: dict[str, Any], 
                    headers: dict[str, str], overrides: dict[str, Any]) -> bytes:
        """Build request body"""
        if not operation.request_body:
            return b""
        
        # Handle batch operations specially
        if "SubmitBatch" in operation.operation_id:
            return self._build_batch_body(operation, param_values, headers, overrides)
        
        # For blob uploads, use simple test content
        if operation.operation_id == "BlockBlob_Upload":
            return b"hello world"
        elif operation.operation_id == "PageBlob_UploadPages":
            # Must be 512-byte aligned
            return b"\x00" * 512
        elif operation.operation_id == "AppendBlob_AppendBlock":
            return b"appended content"
        
        # For XML bodies, would need to generate from schema
        # For now, return empty
        return b""
    
    def _build_batch_body(self, operation: Operation, param_values: dict[str, Any],
                          headers: dict[str, str], overrides: dict[str, Any]) -> bytes:
        """Build multipart batch request body for Service_SubmitBatch and Container_SubmitBatch"""
        # Generate batch boundary
        batch_id = str(uuid.uuid4())
        boundary = f"batch_{batch_id}"
        
        # Set Content-Type header with boundary
        headers["Content-Type"] = f"multipart/mixed; boundary={boundary}"
        
        # Get container and blob from overrides (set by setup chain)
        container_name = overrides.get("_containerName", "testcontainer")
        blob_name = overrides.get("_blob", "testblob")
        
        # Build a single sub-request (DELETE blob)
        # The sub-request path for blob operations
        sub_path = f"/{self.account_name}/{container_name}/{blob_name}"
        sub_method = "DELETE"
        
        # Build sub-request headers
        sub_headers = {
            "x-ms-version": "2021-10-04",
            "x-ms-date": self._format_rfc1123(dt.datetime.now(dt.timezone.utc)),
            "Content-Length": "0"
        }
        
        # Compute auth for sub-request
        sub_query_params = []
        sub_body = b""
        sub_auth = self.compute_shared_key(sub_method, sub_path, sub_headers, sub_query_params, sub_body)
        sub_headers["Authorization"] = sub_auth
        
        # Build the sub-request HTTP message
        sub_request_lines = [
            f"{sub_method} {sub_path} HTTP/1.1"
        ]
        for name, value in sub_headers.items():
            sub_request_lines.append(f"{name}: {value}")
        sub_request_lines.append("")  # Empty line after headers
        sub_request = "\r\n".join(sub_request_lines)
        
        # Build multipart body
        parts = []
        parts.append(f"--{boundary}")
        parts.append("Content-Type: application/http")
        parts.append("Content-Transfer-Encoding: binary")
        parts.append("Content-ID: 0")
        parts.append("")
        parts.append(sub_request)
        parts.append(f"--{boundary}--")
        
        batch_body = "\r\n".join(parts) + "\r\n"
        return batch_body.encode("utf-8")
    
    def _update_headers_for_body(self, headers: dict[str, str], body: bytes, method: str):
        """Update headers based on body content and method"""
        # Always set Content-Length for methods that may have a body.
        # http.client adds it automatically, so the signature must match.
        if method in ("PUT", "POST", "DELETE"):
            headers["Content-Length"] = str(len(body))
    
    def compute_shared_key(
        self, method: str, path: str, headers: dict[str, str], 
        query_params: list[tuple[str, str]], body: bytes = b""
    ) -> str:
        """Compute SharedKey Authorization header value.
        
        Follows the Azure SDK's exact string-to-sign construction:
        - 12 header fields joined with \\n, then \\n
        - Canonicalized x-ms-* headers (each line ending with \\n)
        - Canonicalized resource (/{account}{path} with query params)
        """
        # 12 header fields
        header_fields = [
            method.upper(),
            headers.get("Content-Encoding", ""),
            headers.get("Content-Language", ""),
            self._content_length_for_signature(headers, body),
            headers.get("Content-MD5", ""),
            headers.get("Content-Type", ""),
            headers.get("Date", ""),
            headers.get("If-Modified-Since", ""),
            headers.get("If-Match", ""),
            headers.get("If-None-Match", ""),
            headers.get("If-Unmodified-Since", ""),
            headers.get("Range", ""),
        ]
        
        # SDK format: array.join("\n") + "\n" + canonHeaders + canonResource
        string_to_sign = (
            "\n".join(header_fields) + "\n"
            + self._canonicalized_x_ms_headers(headers)
            + self._canonicalized_resource(path, query_params)
        )
        
        digest = hmac.new(
            base64.b64decode(self.account_key),
            string_to_sign.encode("utf-8"),
            hashlib.sha256,
        ).digest()
        
        signature = base64.b64encode(digest).decode("ascii")
        return f"SharedKey {self.account_name}:{signature}"
    
    def _content_length_for_signature(self, headers: dict[str, str], body: bytes) -> str:
        """Get content-length for signature (empty string if 0)"""
        if "Content-Length" in headers:
            try:
                length = int(headers["Content-Length"])
                return "" if length == 0 else str(length)
            except ValueError:
                return headers["Content-Length"]
        
        length = len(body)
        return "" if length == 0 else str(length)
    
    def _canonicalized_x_ms_headers(self, headers: dict[str, str]) -> str:
        """Build canonicalized x-ms-* headers. Each header line ends with \\n."""
        x_ms_headers = {}
        for name, value in headers.items():
            lower_name = name.lower()
            if lower_name.startswith("x-ms-"):
                normalized = " ".join(value.strip().split())
                x_ms_headers[lower_name] = normalized
        
        # Each header on its own line, each ending with \n
        return "".join(f"{name}:{value}\n" for name, value in sorted(x_ms_headers.items()))
    
    def _canonicalized_resource(
        self, path: str, query_params: list[tuple[str, str]]
    ) -> str:
        """Build canonicalized resource.
        
        For emulator URLs, path already contains /{accountName}/...,
        and the spec requires /{accountName}{path}, resulting in the
        account name appearing twice: /{account}/{account}/container.
        """
        # path already has /{accountName}/... from _build_path
        # Prepend /{accountName} again per Azure SharedKey spec
        canonical = f"/{self.account_name}{path}"
        
        if not query_params:
            return canonical
        
        # Group query params by key (lowercase)
        grouped: dict[str, list[str]] = {}
        for key, value in query_params:
            grouped.setdefault(key.lower(), []).append(urllib.parse.unquote(str(value)))
        
        # Sort and format
        lines = [canonical]
        for key in sorted(grouped):
            lines.append(f"{key}:{','.join(sorted(grouped[key]))}")
        
        return "\n".join(lines)
    
    def _random_suffix(self) -> str:
        """Generate random suffix for resource names"""
        return "".join(random.choices(string.ascii_lowercase + string.digits, k=8))
    
    def _format_rfc1123(self, moment: dt.datetime) -> str:
        """Format datetime as RFC 1123"""
        return moment.strftime("%a, %d %b %Y %H:%M:%S GMT")


@dataclass
class Request:
    """Complete HTTP request"""
    method: str
    url: str
    headers: dict[str, str]
    body: bytes
    path: str
    query_params: list[tuple[str, str]]
    
    def __str__(self) -> str:
        lines = [f"{self.method} {self.url}"]
        for name, value in sorted(self.headers.items()):
            lines.append(f"{name}: {value}")
        if self.body:
            lines.append(f"\nBody: {len(self.body)} bytes")
            if len(self.body) < 100:
                lines.append(f"  {self.body!r}")
        return "\n".join(lines)


if __name__ == "__main__":
    import os
    
    # Find swagger spec
    script_dir = os.path.dirname(os.path.abspath(__file__))
    repo_root = os.path.dirname(os.path.dirname(os.path.dirname(script_dir)))
    spec_path = os.path.join(repo_root, "swagger", "blob-storage-2021-10-04.json")
    
    print("=== Azure Blob Storage Swagger Parser ===\n")
    print(f"Loading spec from: {spec_path}\n")
    
    # Load spec
    spec = SwaggerSpec(spec_path)
    
    # Print operations by tier
    operations = spec.get_operations()
    print(f"Total operations: {len(operations)}\n")
    
    by_tier: dict[int, list[Operation]] = {}
    for op in operations:
        by_tier.setdefault(op.tier, []).append(op)
    
    for tier in sorted(by_tier.keys()):
        ops = by_tier[tier]
        print(f"TIER {tier} ({len(ops)} operations):")
        for op in ops:
            print(f"  {op.operation_id:40} {op.method:6} {op.group}")
        print()
    
    # Show dependency chain example
    print("=== Dependency Chain Example ===")
    example_op = "Blob_SetMetadata"
    chain = spec.get_dependency_chain(example_op)
    print(f"\n{example_op} requires:")
    for i, op in enumerate(chain, 1):
        print(f"  {i}. {op.operation_id} ({op.description[:60]}...)")
    
    # Build sample request
    print("\n=== Sample Request: Container_Create ===\n")
    builder = RequestBuilder(
        account_name="devstoreaccount1",
        account_key="Eby8vdM02xNOcqFlqUwJPLlmEtlCDXJ1OUzFT50uSRZ6IFsuFq2UVErCz4I6tq/K1SZFPTOtr/KBHBeksoGMGw==",
        host="127.0.0.1",
        port=10000
    )
    
    container_create = spec.get_operation("Container_Create")
    if container_create:
        request = builder.build_request(container_create)
        print(request)
    
    print("\n=== Sample Request: BlockBlob_Upload ===\n")
    blob_upload = spec.get_operation("BlockBlob_Upload")
    if blob_upload:
        request = builder.build_request(
            blob_upload, 
            overrides={"_containerName": "testcontainer", "_blob": "testblob.txt"}
        )
        print(request)
