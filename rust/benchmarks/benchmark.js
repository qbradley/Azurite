#!/usr/bin/env node
// rust/benchmarks/benchmark.js
//
// Performance benchmark for Azurite (TS vs Rust, disk vs in-memory).
// Reproduces the write-then-read pattern from the in-memory-persistence design doc
// and adds startup time, memory, and CPU measurements.
//
// Usage: node benchmark.js --impl <ts|rust> --mode <memory|disk> [--iterations N] [--warmup N]

"use strict";

const { BlobServiceClient } = require("@azure/storage-blob");
const { QueueServiceClient } = require("@azure/storage-queue");
const { TableClient, TableServiceClient } = require("@azure/data-tables");
const { spawn } = require("child_process");
const fs = require("fs");
const path = require("path");
const os = require("os");

// ---------------------------------------------------------------------------
// Configuration
// ---------------------------------------------------------------------------

const BLOB_PORT = 13000;
const QUEUE_PORT = 13001;
const TABLE_PORT = 13002;
const ACCOUNT = "devstoreaccount1";
const ACCOUNT_KEY =
  "Eby8vdM02xNOcqFlqUwJPLlmEtlCDXJ1OUzFT50uSRZ6IFsuFq2UVErCz4I6tq/K1SZFPTOtr/KBHBeksoGMGw==";
const CONNECTION_STRING =
  `DefaultEndpointsProtocol=http;AccountName=${ACCOUNT};AccountKey=${ACCOUNT_KEY};` +
  `BlobEndpoint=http://127.0.0.1:${BLOB_PORT}/${ACCOUNT};` +
  `QueueEndpoint=http://127.0.0.1:${QUEUE_PORT}/${ACCOUNT};` +
  `TableEndpoint=http://127.0.0.1:${TABLE_PORT}/${ACCOUNT};`;

const PAGE_SIZE = 4096; // Linux default
const DATA_8K = Buffer.alloc(8 * 1024, 0x61); // 8 KiB of 'a'

// ---------------------------------------------------------------------------
// CLI argument parsing
// ---------------------------------------------------------------------------

function parseArgs() {
  const args = process.argv.slice(2);
  const opts = {
    impl: null,       // "ts" or "rust"
    mode: null,       // "memory" or "disk"
    iterations: 200,
    warmup: 20,
    json: false,      // output JSON instead of table
  };
  for (let i = 0; i < args.length; i++) {
    switch (args[i]) {
      case "--impl":
        opts.impl = args[++i];
        break;
      case "--mode":
        opts.mode = args[++i];
        break;
      case "--iterations":
        opts.iterations = parseInt(args[++i], 10);
        break;
      case "--warmup":
        opts.warmup = parseInt(args[++i], 10);
        break;
      case "--json":
        opts.json = true;
        break;
    }
  }
  if (!opts.impl || !opts.mode) {
    console.error("Usage: node benchmark.js --impl <ts|rust> --mode <memory|disk> [--iterations N] [--warmup N] [--json]");
    process.exit(1);
  }
  return opts;
}

// ---------------------------------------------------------------------------
// Process metrics sampling
// ---------------------------------------------------------------------------

class ProcessMetrics {
  constructor(pid) {
    this.pid = pid;
    this.samples = [];
    this._interval = null;
    this._prevCpu = null;
  }

  start(intervalMs = 200) {
    this._interval = setInterval(() => this._sample(), intervalMs);
    this._sample(); // immediate first sample
  }

  stop() {
    if (this._interval) clearInterval(this._interval);
  }

  _sample() {
    try {
      const statRaw = fs.readFileSync(`/proc/${this.pid}/stat`, "utf8");
      // Field 2 (comm) is in parens and may contain spaces; skip past it
      const closeParen = statRaw.lastIndexOf(")");
      const fields = statRaw.substring(closeParen + 2).split(" ");
      // After stripping pid and (comm), fields[0] = state (field 3)
      // RSS is field 24 in the full stat (1-based), which is fields[21] here
      const rssPages = parseInt(fields[21], 10);
      const rssBytes = rssPages * PAGE_SIZE;
      // utime = field 14 (1-based) = fields[11], stime = field 15 = fields[12]
      const utime = parseInt(fields[11], 10);
      const stime = parseInt(fields[12], 10);
      const totalTicks = utime + stime;
      const now = process.hrtime.bigint();

      let cpuPercent = 0;
      if (this._prevCpu) {
        const dtNs = Number(now - this._prevCpu.time);
        const dtTicks = totalTicks - this._prevCpu.ticks;
        const tickHz = 100; // sysconf(_SC_CLK_TCK) is typically 100
        cpuPercent = (dtTicks / tickHz) / (dtNs / 1e9) * 100;
      }
      this._prevCpu = { time: now, ticks: totalTicks };

      this.samples.push({ time: now, rssBytes, cpuPercent });
    } catch {
      // Process may have exited
    }
  }

  summary() {
    if (this.samples.length === 0) return { peakMemMB: 0, medianMemMB: 0, avgCpu: 0, peakCpu: 0 };
    const rssSorted = this.samples.map((s) => s.rssBytes).sort((a, b) => a - b);
    const cpuSamples = this.samples.filter((s) => s.cpuPercent > 0).map((s) => s.cpuPercent);
    const median = rssSorted[Math.floor(rssSorted.length / 2)];
    const peak = rssSorted[rssSorted.length - 1];
    const avgCpu = cpuSamples.length > 0
      ? cpuSamples.reduce((a, b) => a + b, 0) / cpuSamples.length
      : 0;
    const peakCpu = cpuSamples.length > 0 ? Math.max(...cpuSamples) : 0;
    return {
      peakMemMB: peak / (1024 * 1024),
      medianMemMB: median / (1024 * 1024),
      avgCpu: avgCpu,
      peakCpu: peakCpu,
      sampleCount: this.samples.length,
    };
  }
}

// ---------------------------------------------------------------------------
// Server lifecycle
// ---------------------------------------------------------------------------

function findRustBinary() {
  const repoRoot = path.resolve(__dirname, "../..");
  const candidates = [
    path.join(repoRoot, "rust/target/release/azurite"),
    path.join(repoRoot, "rust/target/x86_64-unknown-linux-gnu/release/azurite"),
  ];
  for (const c of candidates) {
    if (fs.existsSync(c)) return c;
  }
  throw new Error("Rust azurite binary not found. Run: cd rust && cargo build --release");
}

function startServer(impl, mode) {
  const repoRoot = path.resolve(__dirname, "../..");
  const isMemory = mode === "memory";
  const tmpDir = fs.mkdtempSync(path.join(os.tmpdir(), "azurite-bench-"));

  let proc;
  if (impl === "ts") {
    const args = [
      path.join(repoRoot, "dist/src/azurite.js"),
      "--blobHost", "127.0.0.1", "--blobPort", String(BLOB_PORT),
      "--queueHost", "127.0.0.1", "--queuePort", String(QUEUE_PORT),
      "--tableHost", "127.0.0.1", "--tablePort", String(TABLE_PORT),
      "--silent",
    ];
    if (isMemory) {
      args.push("--inMemoryPersistence");
    } else {
      args.push("--location", tmpDir);
    }
    proc = spawn("node", args, {
      cwd: repoRoot,
      stdio: ["ignore", "pipe", "pipe"],
      env: { ...process.env },
    });
  } else {
    const bin = findRustBinary();
    const args = [
      "--blobHost", "127.0.0.1", "--blobPort", String(BLOB_PORT),
      "--queueHost", "127.0.0.1", "--queuePort", String(QUEUE_PORT),
      "--tableHost", "127.0.0.1", "--tablePort", String(TABLE_PORT),
    ];
    if (isMemory) {
      args.push("--inMemoryPersistence");
    } else {
      args.push("--location", tmpDir);
    }
    proc = spawn(bin, args, {
      cwd: repoRoot,
      stdio: ["ignore", "pipe", "pipe"],
      env: {
        ...process.env,
        AZURITE_ACCOUNTS: `${ACCOUNT}:${ACCOUNT_KEY}`,
        RUST_LOG: "warn",
      },
    });
  }

  return { proc, tmpDir };
}

async function waitForReady(startTimeNs) {
  const http = require("http");
  const endpoints = [
    `http://127.0.0.1:${BLOB_PORT}/${ACCOUNT}?comp=properties`,
    `http://127.0.0.1:${QUEUE_PORT}/${ACCOUNT}?comp=properties`,
    `http://127.0.0.1:${TABLE_PORT}/${ACCOUNT}/Tables`,
  ];

  for (let attempt = 0; attempt < 60; attempt++) {
    let allReady = true;
    for (const url of endpoints) {
      try {
        await new Promise((resolve, reject) => {
          const req = http.get(url, { timeout: 1000 }, (res) => {
            res.resume();
            resolve(res.statusCode);
          });
          req.on("error", reject);
          req.on("timeout", () => { req.destroy(); reject(new Error("timeout")); });
        });
      } catch {
        allReady = false;
        break;
      }
    }
    if (allReady) {
      const readyTimeNs = process.hrtime.bigint();
      return Number(readyTimeNs - startTimeNs) / 1e6; // milliseconds
    }
    await new Promise((r) => setTimeout(r, 250));
  }
  throw new Error("Server did not become ready within 15 seconds");
}

function killServer(proc) {
  return new Promise((resolve) => {
    if (!proc || proc.exitCode !== null) { resolve(); return; }
    proc.on("exit", resolve);
    proc.kill("SIGTERM");
    setTimeout(() => {
      try { proc.kill("SIGKILL"); } catch { /* ignore */ }
      resolve();
    }, 3000);
  });
}

// ---------------------------------------------------------------------------
// Benchmark operations (matching the gist)
// ---------------------------------------------------------------------------

async function benchmarkBlob(blobContainer, iterations) {
  const timings = [];
  for (let i = 0; i < iterations; i++) {
    const start = process.hrtime.bigint();
    const blob = blobContainer.getBlockBlobClient("testblob");
    await blob.upload(DATA_8K, DATA_8K.length, { overwrite: true });
    await blob.setMetadata({ foo: "bar" });
    await blob.delete();
    const end = process.hrtime.bigint();
    timings.push(Number(end - start) / 1e3); // microseconds
  }
  return timings;
}

async function benchmarkQueue(queue, iterations) {
  const timings = [];
  for (let i = 0; i < iterations; i++) {
    const start = process.hrtime.bigint();
    await queue.sendMessage(DATA_8K.toString("base64"));
    const resp = await queue.receiveMessages();
    const msg = resp.receivedMessageItems[0];
    await queue.deleteMessage(msg.messageId, msg.popReceipt);
    const end = process.hrtime.bigint();
    timings.push(Number(end - start) / 1e3);
  }
  return timings;
}

async function benchmarkTable(table, iterations) {
  const timings = [];
  for (let i = 0; i < iterations; i++) {
    const start = process.hrtime.bigint();
    await table.upsertEntity({
      partitionKey: "pk",
      rowKey: "rk",
      data: DATA_8K.toString("base64"),
    });
    const entities = table.listEntities({ queryOptions: { filter: "PartitionKey eq 'pk'" } });
    for await (const _e of entities) { /* consume */ }
    const end = process.hrtime.bigint();
    timings.push(Number(end - start) / 1e3);
  }
  return timings;
}

// ---------------------------------------------------------------------------
// Statistics helpers
// ---------------------------------------------------------------------------

function computeStats(timingsUs) {
  const sorted = [...timingsUs].sort((a, b) => a - b);
  const n = sorted.length;
  const mean = sorted.reduce((a, b) => a + b, 0) / n;
  const median = sorted[Math.floor(n / 2)];
  const p95 = sorted[Math.floor(n * 0.95)];
  const p99 = sorted[Math.floor(n * 0.99)];
  const min = sorted[0];
  const max = sorted[n - 1];
  const variance = sorted.reduce((sum, v) => sum + (v - mean) ** 2, 0) / n;
  const stddev = Math.sqrt(variance);
  return { mean, median, p95, p99, min, max, stddev, count: n };
}

function formatUs(us) {
  if (us >= 1e6) return `${(us / 1e6).toFixed(1)} s`;
  if (us >= 1e3) return `${(us / 1e3).toFixed(1)} ms`;
  return `${us.toFixed(0)} µs`;
}

// ---------------------------------------------------------------------------
// Main
// ---------------------------------------------------------------------------

async function main() {
  const opts = parseArgs();
  const label = `${opts.impl}/${opts.mode}`;

  console.error(`\n=== Benchmark: ${label} (${opts.iterations} iterations, ${opts.warmup} warmup) ===\n`);

  // Start server
  const startTimeNs = process.hrtime.bigint();
  const { proc, tmpDir } = startServer(opts.impl, opts.mode);

  // Capture stderr for debugging
  let stderr = "";
  proc.stderr.on("data", (d) => { stderr += d.toString(); });
  proc.stdout.on("data", () => {}); // drain

  let metrics;
  try {
    // Wait for ready
    console.error("Waiting for server...");
    const startupMs = await waitForReady(startTimeNs);
    console.error(`Server ready in ${startupMs.toFixed(0)} ms (PID ${proc.pid})`);

    // Start metrics sampling
    metrics = new ProcessMetrics(proc.pid);
    metrics.start(200);

    // Initialize clients
    const blobService = BlobServiceClient.fromConnectionString(CONNECTION_STRING);
    const blobContainer = blobService.getContainerClient("testcontainer");
    await blobContainer.createIfNotExists();

    const queueService = QueueServiceClient.fromConnectionString(CONNECTION_STRING);
    const queue = queueService.getQueueClient("testqueue");
    await queue.createIfNotExists();

    const tableService = TableServiceClient.fromConnectionString(CONNECTION_STRING, { allowInsecureConnection: true });
    await tableService.createTable("testtable").catch(() => {});
    const table = TableClient.fromConnectionString(CONNECTION_STRING, "testtable", { allowInsecureConnection: true });

    // Warmup
    console.error(`Running ${opts.warmup} warmup iterations...`);
    await benchmarkBlob(blobContainer, opts.warmup);
    await benchmarkQueue(queue, opts.warmup);
    await benchmarkTable(table, opts.warmup);

    // Benchmark
    console.error(`Running ${opts.iterations} benchmark iterations...`);
    const blobTimings = await benchmarkBlob(blobContainer, opts.iterations);
    const queueTimings = await benchmarkQueue(queue, opts.iterations);
    const tableTimings = await benchmarkTable(table, opts.iterations);

    metrics.stop();

    const blobStats = computeStats(blobTimings);
    const queueStats = computeStats(queueTimings);
    const tableStats = computeStats(tableTimings);
    const memStats = metrics.summary();

    const result = {
      label,
      impl: opts.impl,
      mode: opts.mode,
      iterations: opts.iterations,
      startupMs: Math.round(startupMs),
      blob: blobStats,
      queue: queueStats,
      table: tableStats,
      memory: memStats,
    };

    if (opts.json) {
      console.log(JSON.stringify(result));
    } else {
      console.log(`\n--- Results: ${label} ---`);
      console.log(`Startup time:       ${startupMs.toFixed(0)} ms`);
      console.log(`Memory (median):    ${memStats.medianMemMB.toFixed(1)} MB`);
      console.log(`Memory (peak):      ${memStats.peakMemMB.toFixed(1)} MB`);
      console.log(`CPU (avg):          ${memStats.avgCpu.toFixed(1)}%`);
      console.log(`CPU (peak):         ${memStats.peakCpu.toFixed(1)}%`);
      console.log(`Samples collected:  ${memStats.sampleCount}`);
      console.log();
      console.log(`| Operation | Mean       | Median     | P95        | P99        | StdDev     |`);
      console.log(`|-----------|------------|------------|------------|------------|------------|`);
      for (const [name, stats] of [["Blob", blobStats], ["Queue", queueStats], ["Table", tableStats]]) {
        console.log(
          `| ${name.padEnd(9)} | ${formatUs(stats.mean).padStart(10)} | ${formatUs(stats.median).padStart(10)} | ${formatUs(stats.p95).padStart(10)} | ${formatUs(stats.p99).padStart(10)} | ${formatUs(stats.stddev).padStart(10)} |`
        );
      }
      console.log();
    }
  } catch (err) {
    console.error(`Benchmark failed: ${err.message}`);
    if (stderr) console.error(`Server stderr:\n${stderr.slice(-500)}`);
    process.exitCode = 1;
  } finally {
    if (metrics) metrics.stop();
    await killServer(proc);
    // Cleanup temp dir
    try { fs.rmSync(tmpDir, { recursive: true, force: true }); } catch { /* ignore */ }
  }
}

main();
