//! Integration tests for Rust Azurite using the Azure Rust SDK (`azure_storage_blobs`).
//!
//! These tests validate that Azurite's HTTP responses are fully compatible with the
//! Azure Rust SDK's deserialization logic, particularly date header formats (RFC 1123),
//! error response parsing, and all major blob API operations.
//!
//! # Running
//! ```bash
//! cd rust && cargo test -p azurite-integration-tests -- --test-threads=1
//! ```

use azure_core::prelude::*;
use azure_storage::prelude::*;
use azure_storage::CloudLocation;
use azure_storage_blobs::prelude::*;
use futures::StreamExt;
use std::process::Command;
use std::sync::OnceLock;
use uuid::Uuid;

// ---------------------------------------------------------------------------
// Test harness: server lifecycle management
// ---------------------------------------------------------------------------

const BLOB_PORT: u16 = 13000;
const QUEUE_PORT: u16 = 13001;
const TABLE_PORT: u16 = 13002;
const ACCOUNT_NAME: &str = "devstoreaccount1";
const ACCOUNT_KEY: &str =
    "Eby8vdM02xNOcqFlqUwJPLlmEtlCDXJ1OUzFT50uSRZ6IFsuFq2UVErCz4I6tq/K1SZFPTOtr/KBHBeksoGMGw==";

struct TestServer {
    pid: u32,
}

impl Drop for TestServer {
    fn drop(&mut self) {
        let _ = Command::new("kill").arg(self.pid.to_string()).status();
    }
}

static SERVER: OnceLock<TestServer> = OnceLock::new();

fn azurite_binary_path() -> String {
    if let Ok(path) = std::env::var("AZURITE_BINARY") {
        return path;
    }
    let workspace_root =
        env!("CARGO_MANIFEST_DIR").trim_end_matches("/crates/azurite-integration-tests");
    let release_path = format!(
        "{}/target/x86_64-unknown-linux-gnu/release/azurite",
        workspace_root
    );
    if std::path::Path::new(&release_path).exists() {
        return release_path;
    }
    format!("{}/target/debug/azurite", workspace_root)
}

fn ensure_server() -> &'static TestServer {
    SERVER.get_or_init(|| {
        let binary = azurite_binary_path();
        assert!(
            std::path::Path::new(&binary).exists(),
            "Azurite binary not found at {}. Build with: cargo build --release --target x86_64-unknown-linux-gnu",
            binary
        );

        #[allow(clippy::zombie_processes)]
        let child = Command::new(&binary)
            .args([
                "--blobHost",
                "127.0.0.1",
                "--blobPort",
                &BLOB_PORT.to_string(),
                "--queueHost",
                "127.0.0.1",
                "--queuePort",
                &QUEUE_PORT.to_string(),
                "--tableHost",
                "127.0.0.1",
                "--tablePort",
                &TABLE_PORT.to_string(),
                "--inMemoryPersistence",
                "--skipApiVersionCheck",
                "--silent",
            ])
            .stdout(std::process::Stdio::null())
            .stderr(std::process::Stdio::null())
            .spawn()
            .unwrap_or_else(|e| panic!("Failed to start Azurite at {}: {}", binary, e));

        let pid = child.id();

        // Wait for server to be ready using a raw TCP connection check
        // (avoids reqwest::blocking which conflicts with tokio::test runtime)
        let addr = format!("127.0.0.1:{}", BLOB_PORT);
        let start = std::time::Instant::now();
        loop {
            if start.elapsed() > std::time::Duration::from_secs(30) {
                panic!("Azurite failed to start within 30 seconds");
            }
            match std::net::TcpStream::connect(&addr) {
                Ok(_) => break,
                Err(_) => std::thread::sleep(std::time::Duration::from_millis(100)),
            }
        }

        TestServer { pid }
    })
}

fn blob_service_url() -> String {
    format!("http://127.0.0.1:{}/devstoreaccount1", BLOB_PORT)
}

fn make_client() -> ClientBuilder {
    let creds = StorageCredentials::access_key(ACCOUNT_NAME, ACCOUNT_KEY);
    let location = CloudLocation::Custom {
        account: ACCOUNT_NAME.to_string(),
        uri: blob_service_url(),
    };
    ClientBuilder::with_location(location, creds)
}

fn container_client(name: &str) -> ContainerClient {
    make_client().container_client(name)
}

fn unique_container_name() -> String {
    format!(
        "test-{}",
        Uuid::new_v4().to_string().split('-').next().unwrap()
    )
}

async fn create_test_container() -> (ContainerClient, String) {
    let name = unique_container_name();
    let client = container_client(&name);
    client
        .create()
        .into_future()
        .await
        .expect("create container");
    (client, name)
}

// ---------------------------------------------------------------------------
// Container Operations
// ---------------------------------------------------------------------------

#[tokio::test]
async fn test_create_and_delete_container() {
    ensure_server();
    let name = unique_container_name();
    let client = container_client(&name);

    let response = client.create().into_future().await;
    assert!(
        response.is_ok(),
        "create container failed: {:?}",
        response.err()
    );

    let response = client.delete().into_future().await;
    assert!(
        response.is_ok(),
        "delete container failed: {:?}",
        response.err()
    );
}

#[tokio::test]
async fn test_get_container_properties() {
    ensure_server();
    let (client, _name) = create_test_container().await;

    let props = client.get_properties().into_future().await;
    assert!(
        props.is_ok(),
        "get container properties failed: {:?}",
        props.err()
    );
    // SDK successfully parsed the response including date headers
    let props = props.unwrap();
    assert!(
        props.date.unix_timestamp() > 0,
        "response date should be valid"
    );

    let _ = client.delete().into_future().await;
}

#[tokio::test]
async fn test_list_containers() {
    ensure_server();
    let (client, name) = create_test_container().await;

    let svc = make_client().blob_service_client();
    let mut found = false;
    let mut stream = svc.list_containers().into_stream();
    while let Some(response) = stream.next().await {
        let response = response.expect("list containers page");
        for c in response.containers {
            if c.name == name {
                found = true;
            }
        }
    }
    assert!(found, "created container '{}' not found in list", name);

    let _ = client.delete().into_future().await;
}

// ---------------------------------------------------------------------------
// Block Blob Operations
// ---------------------------------------------------------------------------

#[tokio::test]
async fn test_upload_and_download_block_blob() {
    ensure_server();
    let (cc, _name) = create_test_container().await;
    let blob = cc.blob_client("test-blob.txt");

    let data = b"Hello from the Azure Rust SDK!";
    blob.put_block_blob(data.to_vec())
        .into_future()
        .await
        .expect("upload blob");

    let mut stream = blob.get().into_stream();
    let mut result: Vec<u8> = Vec::new();
    while let Some(chunk) = stream.next().await {
        let chunk = chunk.expect("download chunk");
        let mut body = chunk.data;
        while let Some(bytes) = body.next().await {
            result.extend(&bytes.expect("body bytes"));
        }
    }
    assert_eq!(result, data, "downloaded content mismatch");

    let _ = cc.delete().into_future().await;
}

#[tokio::test]
async fn test_upload_blob_with_content_type() {
    ensure_server();
    let (cc, _name) = create_test_container().await;
    let blob = cc.blob_client("typed-blob.json");

    blob.put_block_blob(b"{\"key\": \"value\"}".to_vec())
        .content_type("application/json")
        .into_future()
        .await
        .expect("upload with content type");

    let props = blob
        .get_properties()
        .into_future()
        .await
        .expect("get blob properties");
    assert_eq!(
        props.blob.properties.content_type, "application/json",
        "content type mismatch"
    );

    let _ = cc.delete().into_future().await;
}

/// This is the core regression test for the RFC 1123 date format bug.
/// If the server returns ISO 8601 dates, the SDK's date parser will fail
/// and this test will panic with a deserialization error.
#[tokio::test]
async fn test_blob_properties_have_valid_dates() {
    ensure_server();
    let (cc, _name) = create_test_container().await;
    let blob = cc.blob_client("date-test-blob.txt");

    blob.put_block_blob(b"date check".to_vec())
        .into_future()
        .await
        .expect("upload");

    let props = blob.get_properties().into_future().await.expect(
        "get_properties should succeed - if this fails, date headers may not be in RFC 1123 format",
    );

    let last_modified = props.blob.properties.last_modified;
    assert!(
        last_modified.unix_timestamp() > 0,
        "last_modified should be a valid date, got: {:?}",
        last_modified
    );

    let creation_time = props.blob.properties.creation_time;
    assert!(
        creation_time.unix_timestamp() > 0,
        "creation_time should be a valid date, got: {:?}",
        creation_time
    );

    let _ = cc.delete().into_future().await;
}

/// Regression test: dates must remain RFC 1123 after blob modification.
/// The original bug was that set_datetime() in the metadata store
/// re-serialized dates as ISO 8601 when blobs were modified.
#[tokio::test]
async fn test_blob_dates_valid_after_modification() {
    ensure_server();
    let (cc, _name) = create_test_container().await;
    let blob = cc.blob_client("modify-date-test.txt");

    // Create blob
    blob.put_block_blob(b"original".to_vec())
        .into_future()
        .await
        .expect("upload");

    // Modify blob (overwrite) — triggers set_datetime in metadata store
    blob.put_block_blob(b"modified".to_vec())
        .into_future()
        .await
        .expect("overwrite");

    // If set_datetime produced ISO 8601, this will fail
    let props = blob.get_properties().into_future().await.expect(
        "get_properties after modification should succeed - dates must remain RFC 1123 after blob update",
    );

    assert!(
        props.blob.properties.last_modified.unix_timestamp() > 0,
        "last_modified after modification should be valid"
    );

    let _ = cc.delete().into_future().await;
}

/// Regression test: dates valid after set_metadata (another modification path)
#[tokio::test]
async fn test_blob_dates_valid_after_set_metadata() {
    ensure_server();
    let (cc, _name) = create_test_container().await;
    let blob = cc.blob_client("metadata-date-test.txt");

    blob.put_block_blob(b"metadata test".to_vec())
        .into_future()
        .await
        .expect("upload");

    // Set metadata — triggers set_datetime in metadata store
    let mut metadata = Metadata::new();
    metadata.insert("testkey".to_string(), bytes::Bytes::from("testvalue"));
    blob.set_metadata()
        .metadata(metadata)
        .into_future()
        .await
        .expect("set metadata");

    // Dates should still be parseable by SDK
    let props =
        blob.get_properties().into_future().await.expect(
            "get_properties after set_metadata should succeed - dates must remain RFC 1123",
        );

    assert!(
        props.blob.properties.last_modified.unix_timestamp() > 0,
        "last_modified after set_metadata should be valid"
    );

    let _ = cc.delete().into_future().await;
}

#[tokio::test]
async fn test_blob_metadata() {
    ensure_server();
    let (cc, _name) = create_test_container().await;
    let blob = cc.blob_client("metadata-blob.txt");

    blob.put_block_blob(b"metadata test".to_vec())
        .into_future()
        .await
        .expect("upload");

    let mut metadata = Metadata::new();
    metadata.insert("author".to_string(), bytes::Bytes::from("rust-sdk-test"));
    metadata.insert("version".to_string(), bytes::Bytes::from("1"));
    blob.set_metadata()
        .metadata(metadata)
        .into_future()
        .await
        .expect("set metadata");

    let resp = blob
        .get_metadata()
        .into_future()
        .await
        .expect("get metadata");
    let author = resp
        .metadata
        .get("author")
        .map(|v| std::str::from_utf8(v.as_ref()).unwrap().to_string());
    let version = resp
        .metadata
        .get("version")
        .map(|v| std::str::from_utf8(v.as_ref()).unwrap().to_string());
    assert_eq!(author.as_deref(), Some("rust-sdk-test"));
    assert_eq!(version.as_deref(), Some("1"));

    let _ = cc.delete().into_future().await;
}

#[tokio::test]
async fn test_delete_blob() {
    ensure_server();
    let (cc, _name) = create_test_container().await;
    let blob = cc.blob_client("delete-me.txt");

    blob.put_block_blob(b"to be deleted".to_vec())
        .into_future()
        .await
        .expect("upload");

    blob.delete().into_future().await.expect("delete blob");

    let result = blob.get_properties().into_future().await;
    assert!(result.is_err(), "blob should not exist after deletion");

    let _ = cc.delete().into_future().await;
}

#[tokio::test]
async fn test_list_blobs() {
    ensure_server();
    let (cc, _name) = create_test_container().await;

    for i in 0..5 {
        let blob = cc.blob_client(format!("blob-{}.txt", i));
        blob.put_block_blob(format!("content {}", i).into_bytes())
            .into_future()
            .await
            .expect("upload");
    }

    let mut names: Vec<String> = Vec::new();
    let mut stream = cc.list_blobs().into_stream();
    while let Some(response) = stream.next().await {
        let response = response.expect("list blobs page");
        for blob in response.blobs.blobs() {
            names.push(blob.name.clone());
        }
    }

    for i in 0..5 {
        assert!(
            names.contains(&format!("blob-{}.txt", i)),
            "blob-{}.txt not found in list: {:?}",
            i,
            names
        );
    }

    let _ = cc.delete().into_future().await;
}

#[tokio::test]
async fn test_blob_snapshot() {
    ensure_server();
    let (cc, _name) = create_test_container().await;
    let blob = cc.blob_client("snapshot-blob.txt");

    blob.put_block_blob(b"original content".to_vec())
        .into_future()
        .await
        .expect("upload");

    let snap = blob
        .snapshot()
        .into_future()
        .await
        .expect("create snapshot");
    // Snapshot was successfully parsed by SDK (datetime string)
    let _ = snap.snapshot;

    let _ = cc.delete().into_future().await;
}

#[tokio::test]
async fn test_copy_blob() {
    ensure_server();
    let (cc, _name) = create_test_container().await;
    let src_blob = cc.blob_client("source-blob.txt");
    let dst_blob = cc.blob_client("dest-blob.txt");

    src_blob
        .put_block_blob(b"copy me".to_vec())
        .into_future()
        .await
        .expect("upload source");

    let src_url = src_blob.url().expect("source URL");
    dst_blob
        .copy(src_url)
        .into_future()
        .await
        .expect("copy blob");

    let mut stream = dst_blob.get().into_stream();
    let mut result: Vec<u8> = Vec::new();
    while let Some(chunk) = stream.next().await {
        let chunk = chunk.expect("download chunk");
        let mut body = chunk.data;
        while let Some(bytes) = body.next().await {
            result.extend(&bytes.expect("body bytes"));
        }
    }
    assert_eq!(result, b"copy me", "copied content mismatch");

    let _ = cc.delete().into_future().await;
}

/// Regression test: copy blob triggers apply_copy_properties which uses
/// set_datetime. Dates must remain RFC 1123 after copy.
#[tokio::test]
async fn test_copy_blob_dates_valid() {
    ensure_server();
    let (cc, _name) = create_test_container().await;
    let src = cc.blob_client("copy-date-src.txt");
    let dst = cc.blob_client("copy-date-dst.txt");

    src.put_block_blob(b"copy date test".to_vec())
        .into_future()
        .await
        .expect("upload source");

    dst.copy(src.url().unwrap())
        .into_future()
        .await
        .expect("copy");

    let props = dst.get_properties().into_future().await.expect(
        "get_properties after copy should succeed - dates must be RFC 1123 after copy operation",
    );

    assert!(
        props.blob.properties.last_modified.unix_timestamp() > 0,
        "last_modified after copy should be valid"
    );

    let _ = cc.delete().into_future().await;
}

// ---------------------------------------------------------------------------
// Block Blob - Staged Upload (commit block list)
// ---------------------------------------------------------------------------

#[tokio::test]
async fn test_stage_and_commit_block_list() {
    ensure_server();
    let (cc, _name) = create_test_container().await;
    let blob = cc.blob_client("staged-blob.txt");

    let block_id_1 = "block000001";
    let block_id_2 = "block000002";

    blob.put_block(block_id_1, b"Hello ".to_vec())
        .into_future()
        .await
        .expect("stage block 1");

    blob.put_block(block_id_2, b"World!".to_vec())
        .into_future()
        .await
        .expect("stage block 2");

    let block_list = blob
        .get_block_list()
        .block_list_type(BlockListType::All)
        .into_future()
        .await
        .expect("get block list");
    let uncommitted_count = block_list
        .block_with_size_list
        .blocks
        .iter()
        .filter(|b| matches!(b.block_list_type, BlobBlockType::Uncommitted(_)))
        .count();
    assert_eq!(uncommitted_count, 2, "should have 2 uncommitted blocks");

    let block_list = BlockList {
        blocks: vec![
            BlobBlockType::new_latest(block_id_1),
            BlobBlockType::new_latest(block_id_2),
        ],
    };
    blob.put_block_list(block_list)
        .into_future()
        .await
        .expect("commit block list");

    let mut stream = blob.get().into_stream();
    let mut result: Vec<u8> = Vec::new();
    while let Some(chunk) = stream.next().await {
        let chunk = chunk.expect("download chunk");
        let mut body = chunk.data;
        while let Some(bytes) = body.next().await {
            result.extend(&bytes.expect("body bytes"));
        }
    }
    assert_eq!(result, b"Hello World!", "staged upload content mismatch");

    let _ = cc.delete().into_future().await;
}

/// Regression test: commit_block_list triggers set_datetime for lastModified.
#[tokio::test]
async fn test_commit_block_list_dates_valid() {
    ensure_server();
    let (cc, _name) = create_test_container().await;
    let blob = cc.blob_client("commit-date-test.txt");

    blob.put_block("blk1", b"data".to_vec())
        .into_future()
        .await
        .expect("stage block");

    let block_list = BlockList {
        blocks: vec![BlobBlockType::new_latest("blk1")],
    };
    blob.put_block_list(block_list)
        .into_future()
        .await
        .expect("commit block list");

    let props =
        blob.get_properties().into_future().await.expect(
            "get_properties after commit_block_list should succeed - dates must be RFC 1123",
        );

    assert!(
        props.blob.properties.last_modified.unix_timestamp() > 0,
        "last_modified after commit should be valid"
    );

    let _ = cc.delete().into_future().await;
}

// ---------------------------------------------------------------------------
// Blob Leasing
// ---------------------------------------------------------------------------

#[tokio::test]
async fn test_blob_lease_acquire_and_release() {
    ensure_server();
    let (cc, _name) = create_test_container().await;
    let blob = cc.blob_client("leased-blob.txt");

    blob.put_block_blob(b"lease test".to_vec())
        .into_future()
        .await
        .expect("upload");

    let lease = blob
        .acquire_lease(std::time::Duration::from_secs(15))
        .into_future()
        .await
        .expect("acquire lease");

    blob.blob_lease_client(lease.lease_id)
        .release()
        .into_future()
        .await
        .expect("release lease");

    let _ = cc.delete().into_future().await;
}

#[tokio::test]
async fn test_container_lease_acquire_and_release() {
    ensure_server();
    let (client, _name) = create_test_container().await;

    let lease = client
        .acquire_lease(std::time::Duration::from_secs(15))
        .into_future()
        .await
        .expect("acquire container lease");

    client
        .container_lease_client(lease.lease_id)
        .release()
        .into_future()
        .await
        .expect("release container lease");

    let _ = client.delete().into_future().await;
}

/// Regression test: lease must be preserved across put_block_blob overwrites.
/// Customer bug: second put_block_blob with same lease_id fails with
/// LeaseNotPresentWithBlobOperation because Rust Azurite was dropping the lease.
#[tokio::test]
async fn test_lease_preserved_across_put_block_blob() {
    ensure_server();
    let (cc, _name) = create_test_container().await;
    let blob = cc.blob_client("lease-overwrite.txt");

    // Step 1: Create the blob
    blob.put_block_blob(b"initial content".to_vec())
        .into_future()
        .await
        .expect("create blob");

    // Step 2: Acquire an infinite lease
    let lease = blob
        .acquire_lease(azure_core::prelude::LeaseDuration::Infinite)
        .into_future()
        .await
        .expect("acquire lease");
    let lease_id = lease.lease_id;

    // Step 3: Overwrite blob with lease_id — should succeed
    blob.put_block_blob(b"second content".to_vec())
        .lease_id(lease_id.clone())
        .into_future()
        .await
        .expect("first overwrite with lease");

    // Step 4: Overwrite again with same lease_id — this was failing
    blob.put_block_blob(b"third content".to_vec())
        .lease_id(lease_id.clone())
        .into_future()
        .await
        .expect("second overwrite with lease - lease must be preserved");

    // Step 5: Verify the content is correct
    let response = blob
        .get_properties()
        .lease_id(lease_id.clone())
        .into_future()
        .await
        .expect("get properties");
    assert_eq!(
        response.blob.properties.lease_state,
        Some(azure_core::LeaseState::Leased),
        "lease should still be active"
    );

    // Cleanup: release lease and delete
    blob.blob_lease_client(lease_id)
        .release()
        .into_future()
        .await
        .expect("release lease");
    let _ = cc.delete().into_future().await;
}

// ---------------------------------------------------------------------------
// Page Blob Operations
// ---------------------------------------------------------------------------

#[tokio::test]
async fn test_page_blob_create_and_write() {
    ensure_server();
    let (cc, _name) = create_test_container().await;
    let blob = cc.blob_client("page-blob.vhd");

    blob.put_page_blob(1024)
        .into_future()
        .await
        .expect("create page blob");

    // Write a page (512 bytes, must be 512-aligned)
    let data = vec![0xABu8; 512];
    let range = BA512Range::new(0, 511).expect("valid BA512 range");
    blob.put_page(range, data)
        .into_future()
        .await
        .expect("write page");

    let ranges = blob
        .get_page_ranges()
        .into_future()
        .await
        .expect("get page ranges");
    assert!(
        !ranges.page_list.ranges.is_empty(),
        "should have page ranges"
    );

    let props = blob
        .get_properties()
        .into_future()
        .await
        .expect("get properties");
    assert_eq!(props.blob.properties.content_length, 1024);

    let _ = cc.delete().into_future().await;
}

/// Regression test: page blob creation and modification dates must be RFC 1123.
/// This is the exact scenario from the customer bug report.
#[tokio::test]
async fn test_page_blob_dates_valid_after_upload_pages() {
    ensure_server();
    let (cc, _name) = create_test_container().await;
    let blob = cc.blob_client("page-date-test.vhd");

    blob.put_page_blob(1024)
        .into_future()
        .await
        .expect("create page blob");

    let data = vec![0xCDu8; 512];
    let range = BA512Range::new(0, 511).expect("valid range");
    blob.put_page(range, data)
        .into_future()
        .await
        .expect("write page");

    // uploadPages triggers set_datetime in metadata store
    let props = blob.get_properties().into_future().await.expect(
        "get_properties after uploadPages should succeed - this was the customer's exact failure",
    );

    assert!(
        props.blob.properties.last_modified.unix_timestamp() > 0,
        "last_modified after page upload should be valid RFC 1123 date"
    );

    let _ = cc.delete().into_future().await;
}

// ---------------------------------------------------------------------------
// Append Blob Operations
// ---------------------------------------------------------------------------

#[tokio::test]
async fn test_append_blob_create_and_append() {
    ensure_server();
    let (cc, _name) = create_test_container().await;
    let blob = cc.blob_client("append-blob.log");

    blob.put_append_blob()
        .into_future()
        .await
        .expect("create append blob");

    blob.append_block(b"Line 1\n".to_vec())
        .into_future()
        .await
        .expect("append block 1");

    blob.append_block(b"Line 2\n".to_vec())
        .into_future()
        .await
        .expect("append block 2");

    let mut stream = blob.get().into_stream();
    let mut result: Vec<u8> = Vec::new();
    while let Some(chunk) = stream.next().await {
        let chunk = chunk.expect("download chunk");
        let mut body = chunk.data;
        while let Some(bytes) = body.next().await {
            result.extend(&bytes.expect("body bytes"));
        }
    }
    assert_eq!(result, b"Line 1\nLine 2\n", "append blob content mismatch");

    let _ = cc.delete().into_future().await;
}

/// Regression test: append_block triggers set_datetime in metadata store.
#[tokio::test]
async fn test_append_blob_dates_valid_after_append() {
    ensure_server();
    let (cc, _name) = create_test_container().await;
    let blob = cc.blob_client("append-date-test.log");

    blob.put_append_blob()
        .into_future()
        .await
        .expect("create append blob");

    blob.append_block(b"appended data\n".to_vec())
        .into_future()
        .await
        .expect("append block");

    let props =
        blob.get_properties().into_future().await.expect(
            "get_properties after append should succeed - dates must be RFC 1123 after append",
        );

    assert!(
        props.blob.properties.last_modified.unix_timestamp() > 0,
        "last_modified after append should be valid"
    );

    let _ = cc.delete().into_future().await;
}

// ---------------------------------------------------------------------------
// Error Handling
// ---------------------------------------------------------------------------

#[tokio::test]
async fn test_error_container_not_found() {
    ensure_server();
    let client = container_client("nonexistent-container");

    let result = client.get_properties().into_future().await;
    assert!(
        result.is_err(),
        "should return error for nonexistent container"
    );

    let err = result.unwrap_err();
    let err_str = format!("{:?}", err);
    assert!(
        err_str.contains("ContainerNotFound") || err_str.contains("404"),
        "error should indicate container not found, got: {}",
        err_str
    );
}

#[tokio::test]
async fn test_error_blob_not_found() {
    ensure_server();
    let (cc, _name) = create_test_container().await;
    let blob = cc.blob_client("nonexistent-blob.txt");

    let result = blob.get_properties().into_future().await;
    assert!(result.is_err(), "should return error for nonexistent blob");

    let err = result.unwrap_err();
    let err_str = format!("{:?}", err);
    assert!(
        err_str.contains("BlobNotFound") || err_str.contains("404"),
        "error should indicate blob not found, got: {}",
        err_str
    );

    let _ = cc.delete().into_future().await;
}

#[tokio::test]
async fn test_error_container_already_exists() {
    ensure_server();
    let (client, _name) = create_test_container().await;

    let result = client.create().into_future().await;
    assert!(
        result.is_err(),
        "should return error for duplicate container"
    );

    let err = result.unwrap_err();
    let err_str = format!("{:?}", err);
    assert!(
        err_str.contains("ContainerAlreadyExists") || err_str.contains("409"),
        "error should indicate container already exists, got: {}",
        err_str
    );

    let _ = client.delete().into_future().await;
}

#[tokio::test]
async fn test_error_delete_leased_blob_without_lease_id() {
    ensure_server();
    let (cc, _name) = create_test_container().await;
    let blob = cc.blob_client("leased-for-error.txt");

    blob.put_block_blob(b"locked".to_vec())
        .into_future()
        .await
        .expect("upload");

    let lease = blob
        .acquire_lease(std::time::Duration::from_secs(60))
        .into_future()
        .await
        .expect("acquire lease");

    let result = blob.delete().into_future().await;
    assert!(
        result.is_err(),
        "should fail to delete leased blob without lease ID"
    );

    let err = result.unwrap_err();
    let err_str = format!("{:?}", err);
    assert!(
        err_str.contains("LeaseId") || err_str.contains("412"),
        "error should be lease-related, got: {}",
        err_str
    );

    let _ = blob
        .blob_lease_client(lease.lease_id)
        .release()
        .into_future()
        .await;
    let _ = cc.delete().into_future().await;
}

// ---------------------------------------------------------------------------
// Date Header Format Validation (Raw HTTP)
// ---------------------------------------------------------------------------

/// Validates raw HTTP response headers contain RFC 1123 dates.
/// Creates a public-access container to make anonymous HEAD requests,
/// then checks the raw header values against the RFC 1123 format regex.
#[tokio::test]
async fn test_date_headers_are_rfc1123_format() {
    ensure_server();
    let service = make_client().blob_service_client();
    let name = format!("rfc1123pub-{}", uuid::Uuid::new_v4());
    let cc = service.container_client(&name);
    cc.create()
        .public_access(PublicAccess::Blob)
        .into_future()
        .await
        .expect("create public container");

    let blob = cc.blob_client("rfc1123-test.txt");
    blob.put_block_blob(b"date format test".to_vec())
        .into_future()
        .await
        .expect("upload");

    // Anonymous HEAD request (no auth needed for public container)
    let url = format!(
        "http://127.0.0.1:{}/devstoreaccount1/{}/rfc1123-test.txt",
        BLOB_PORT, name
    );

    let http_client = reqwest::Client::new();
    let resp = http_client
        .head(&url)
        .header("x-ms-version", "2024-08-04")
        .send()
        .await
        .expect("HEAD request");

    assert_eq!(
        resp.status().as_u16(),
        200,
        "HEAD request failed: {}",
        resp.status()
    );

    if let Some(date) = resp.headers().get("date") {
        assert_is_rfc1123(date.to_str().unwrap(), "Date");
    }
    if let Some(lm) = resp.headers().get("last-modified") {
        assert_is_rfc1123(lm.to_str().unwrap(), "Last-Modified");
    }
    if let Some(ct) = resp.headers().get("x-ms-creation-time") {
        assert_is_rfc1123(ct.to_str().unwrap(), "x-ms-creation-time");
    }

    let _ = cc.delete().into_future().await;
}

#[tokio::test]
async fn test_list_blobs_date_fields_parsed_by_sdk() {
    ensure_server();
    let (cc, _name) = create_test_container().await;
    let blob = cc.blob_client("list-date-test.txt");

    blob.put_block_blob(b"list date test".to_vec())
        .into_future()
        .await
        .expect("upload");

    let mut stream = cc.list_blobs().into_stream();
    let mut found = false;
    while let Some(response) = stream.next().await {
        let response = response.expect(
            "list_blobs should succeed - if this fails, date fields in XML may not be parseable",
        );
        for blob_item in response.blobs.blobs() {
            if blob_item.name == "list-date-test.txt" {
                found = true;
                assert!(
                    blob_item.properties.last_modified.unix_timestamp() > 0,
                    "list blob last_modified should be valid"
                );
            }
        }
    }
    assert!(found, "blob should appear in listing");

    let _ = cc.delete().into_future().await;
}

// ---------------------------------------------------------------------------
// Large Blob Upload/Download
// ---------------------------------------------------------------------------

#[tokio::test]
async fn test_large_blob_upload_download() {
    ensure_server();
    let (cc, _name) = create_test_container().await;
    let blob = cc.blob_client("large-blob.bin");

    let data: Vec<u8> = (0..1_048_576).map(|i| (i % 256) as u8).collect();
    blob.put_block_blob(data.clone())
        .into_future()
        .await
        .expect("upload large blob");

    let mut stream = blob.get().into_stream();
    let mut result: Vec<u8> = Vec::new();
    while let Some(chunk) = stream.next().await {
        let chunk = chunk.expect("download chunk");
        let mut body = chunk.data;
        while let Some(bytes) = body.next().await {
            result.extend(&bytes.expect("body bytes"));
        }
    }
    assert_eq!(result.len(), data.len(), "large blob size mismatch");
    assert_eq!(result, data, "large blob content mismatch");

    let _ = cc.delete().into_future().await;
}

// ---------------------------------------------------------------------------
// Blob with Special Characters
// ---------------------------------------------------------------------------

#[tokio::test]
async fn test_blob_with_special_characters_in_name() {
    ensure_server();
    let (cc, _name) = create_test_container().await;

    let special_names = [
        "path/to/nested/blob.txt",
        "blob with spaces.txt",
        "blob%20encoded.txt",
    ];

    for name in &special_names {
        let blob = cc.blob_client(*name);
        blob.put_block_blob(format!("content of {}", name).into_bytes())
            .into_future()
            .await
            .unwrap_or_else(|e| panic!("upload '{}' failed: {:?}", name, e));

        let props = blob.get_properties().into_future().await;
        assert!(
            props.is_ok(),
            "get_properties for '{}' failed: {:?}",
            name,
            props.err()
        );
    }

    let _ = cc.delete().into_future().await;
}

// ---------------------------------------------------------------------------
// Service-Level Operations
// ---------------------------------------------------------------------------

#[tokio::test]
async fn test_get_service_properties() {
    ensure_server();
    let svc = make_client().blob_service_client();

    let props = svc.get_properties().into_future().await;
    assert!(
        props.is_ok(),
        "get service properties should succeed: {:?}",
        props.err()
    );
}

// ---------------------------------------------------------------------------
// Conditional Operations
// ---------------------------------------------------------------------------

#[tokio::test]
async fn test_conditional_upload_if_match() {
    ensure_server();
    let (cc, _name) = create_test_container().await;
    let blob = cc.blob_client("conditional-blob.txt");

    blob.put_block_blob(b"first".to_vec())
        .into_future()
        .await
        .expect("first upload");

    let props = blob
        .get_properties()
        .into_future()
        .await
        .expect("get properties");
    let etag = props.blob.properties.etag;

    // Upload with matching etag succeeds
    let result = blob
        .put_block_blob(b"second".to_vec())
        .if_match(IfMatchCondition::Match(etag.to_string()))
        .into_future()
        .await;
    assert!(
        result.is_ok(),
        "conditional upload with matching etag should succeed: {:?}",
        result.err()
    );

    // Upload with stale etag fails (412)
    let result = blob
        .put_block_blob(b"third".to_vec())
        .if_match(IfMatchCondition::Match(etag.to_string()))
        .into_future()
        .await;
    assert!(
        result.is_err(),
        "conditional upload with stale etag should fail"
    );

    let _ = cc.delete().into_future().await;
}

// ---------------------------------------------------------------------------
// Helpers
// ---------------------------------------------------------------------------

fn assert_is_rfc1123(date_str: &str, header_name: &str) {
    let rfc1123_pattern = regex::Regex::new(
        r"^(Mon|Tue|Wed|Thu|Fri|Sat|Sun), \d{2} (Jan|Feb|Mar|Apr|May|Jun|Jul|Aug|Sep|Oct|Nov|Dec) \d{4} \d{2}:\d{2}:\d{2} GMT$"
    ).unwrap();

    assert!(
        rfc1123_pattern.is_match(date_str),
        "{} header '{}' is not in RFC 1123 format (expected: 'Day, DD Mon YYYY HH:MM:SS GMT')",
        header_name,
        date_str
    );
}
