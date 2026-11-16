// Wasm smoke test for the browser playground (Node, no browser needed).
//
// Instantiates the built module exactly as the page does and asserts the
// contract the page relies on: the sample count, the documented frame
// format, per-sample dimensions, byte-for-byte determinism, and the
// dungeon sample's pinned world hash — the same value tools/gate.sh pins
// for the native kit_bridge example, which is the point of the kit:
// native and wasm runs of this code agree bit-for-bit.
//
// Run ./build.sh first, then:  node tests/wasm_smoke.mjs
// A missing .wasm is a failure with instructions, never a silent skip.

import { readFile } from "node:fs/promises";
import { fileURLToPath } from "node:url";
import { dirname, join } from "node:path";

const here = dirname(fileURLToPath(import.meta.url));
const wasmPath = join(here, "..", "web", "izanagi_playground_web.wasm");

let bytes;
try {
  bytes = await readFile(wasmPath);
} catch {
  console.error(`FAIL: ${wasmPath} not found — run ./build.sh in playground-web/ first`);
  process.exit(1);
}

const { instance } = await WebAssembly.instantiate(bytes, {});
const ex = instance.exports;

let failures = 0;
function check(name, cond, detail = "") {
  if (cond) {
    console.log(`ok   ${name}`);
  } else {
    failures++;
    console.error(`FAIL ${name} ${detail}`);
  }
}

function runSample(id, seed) {
  ex.run_sample(id, seed >>> 0, Math.floor(seed / 0x100000000) >>> 0);
  const ptr = ex.frame_ptr();
  const len = ex.frame_len();
  return new Uint8Array(ex.memory.buffer, ptr, len).slice();
}

function parseFrame(buf) {
  const view = new DataView(buf.buffer, buf.byteOffset, buf.byteLength);
  const decoder = new TextDecoder("utf-8");
  let off = 0;
  const u32 = () => { const v = view.getUint32(off, true); off += 4; return v; };
  const w = u32(), h = u32();
  const statusLen = u32();
  const status = decoder.decode(buf.subarray(off, off + statusLen)); off += statusLen;
  const rows = [];
  for (let y = 0; y < h; y++) {
    const glyphLen = u32();
    const glyphs = decoder.decode(buf.subarray(off, off + glyphLen)); off += glyphLen;
    off += w * 3 + w * 3; // fg + bg
    rows.push(glyphs);
  }
  if (off !== buf.byteLength) throw new Error(`frame has ${buf.byteLength - off} trailing bytes`);
  return { w, h, status, rows };
}

const DEFAULT_SEEDS = [0x0DECA1EA, 0x4C011234, 0x1DA57E9];
const EXPECTED_DIMS = [[80, 24], [80, 24], [48, 21]];
const PINNED_DUNGEON_HASH = "353498ec4fbcd160"; // == tools/gate.sh KIT_BRIDGE_HASH

check("sample_count() == 3", ex.sample_count() === 3, `got ${ex.sample_count()}`);

for (let id = 0; id < 3; id++) {
  const first = runSample(id, DEFAULT_SEEDS[id]);
  const second = runSample(id, DEFAULT_SEEDS[id]);
  check(`sample ${id} is byte-for-byte deterministic`, Buffer.from(first).equals(Buffer.from(second)));

  const frame = parseFrame(first);
  const [ew, eh] = EXPECTED_DIMS[id];
  check(`sample ${id} dimensions ${ew}×${eh}`, frame.w === ew && frame.h === eh,
        `got ${frame.w}×${frame.h}`);
  check(`sample ${id} status is non-empty`, frame.status.length > 0);
  check(`sample ${id} every row has ${ew} glyphs`,
        frame.rows.every((r) => [...r].length === ew),
        `row lengths: ${frame.rows.map((r) => [...r].length).join(",")}`);
}

const dungeon = parseFrame(runSample(2, DEFAULT_SEEDS[2]));
check("dungeon status carries the pinned world hash",
      dungeon.status.includes(`final hash ${PINNED_DUNGEON_HASH}`),
      `status: ${dungeon.status}`);
check("dungeon frame shows the player", dungeon.rows.some((r) => r.includes("@")));

const wfc = parseFrame(runSample(1, DEFAULT_SEEDS[1]));
check("wfc left no cell unresolved", wfc.rows.slice(0, 22).every((r) => !r.includes("?")));

const unknown = parseFrame(runSample(99, 0));
check("unknown sample id yields a status-only error frame",
      unknown.w === 0 && unknown.h === 0 && unknown.status.includes("unknown sample id"),
      `status: ${unknown.status}`);

if (failures > 0) {
  console.error(`\n${failures} check(s) failed`);
  process.exit(1);
}
console.log("\nwasm smoke: all checks passed");
