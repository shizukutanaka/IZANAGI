// IZANAGI playground page logic.
//
// Loads izanagi_playground_web.wasm (built by ../build.sh), runs the
// selected sample with the given seed, parses the frame documented in
// ../README.md, and paints the cells. All glyphs reach the DOM through
// textContent — nothing the wasm produces is ever parsed as HTML.

"use strict";

const DEFAULT_SEEDS = [0x0DECA1EA, 0x4C011234, 0x1DA57E9];

let wasm = null;

function parseSeed(text, fallback) {
  const t = String(text).trim();
  if (t === "") return fallback;
  const value = t.toLowerCase().startsWith("0x") ? parseInt(t, 16) : parseInt(t, 10);
  if (!Number.isFinite(value) || value < 0) return null;
  return value >>> 0; // seeds are u32 halves here; the high half stays 0
}

function readU32(view, offset) {
  return view.getUint32(offset, true);
}

function parseFrame(bytes) {
  const view = new DataView(bytes.buffer, bytes.byteOffset, bytes.byteLength);
  const decoder = new TextDecoder("utf-8");
  let off = 0;
  const w = readU32(view, off); off += 4;
  const h = readU32(view, off); off += 4;
  const statusLen = readU32(view, off); off += 4;
  const status = decoder.decode(bytes.subarray(off, off + statusLen)); off += statusLen;
  const rows = [];
  for (let y = 0; y < h; y++) {
    const glyphLen = readU32(view, off); off += 4;
    const glyphs = decoder.decode(bytes.subarray(off, off + glyphLen)); off += glyphLen;
    const fg = bytes.subarray(off, off + w * 3); off += w * 3;
    const bg = bytes.subarray(off, off + w * 3); off += w * 3;
    rows.push({ glyphs: [...glyphs], fg, bg });
  }
  return { w, h, status, rows };
}

function rgb(bytes, i) {
  return `rgb(${bytes[i]},${bytes[i + 1]},${bytes[i + 2]})`;
}

function paint(frame) {
  const screen = document.getElementById("screen");
  screen.replaceChildren();
  for (const row of frame.rows) {
    const rowEl = document.createElement("span");
    rowEl.className = "row";
    row.glyphs.forEach((glyph, x) => {
      const cell = document.createElement("span");
      cell.textContent = glyph;
      cell.style.color = rgb(row.fg, x * 3);
      cell.style.backgroundColor = rgb(row.bg, x * 3);
      rowEl.appendChild(cell);
    });
    screen.appendChild(rowEl);
  }
  document.getElementById("status").textContent = frame.status;
}

function run() {
  const errorEl = document.getElementById("error");
  errorEl.textContent = "";
  const id = Number(document.getElementById("sample").value);
  const seed = parseSeed(document.getElementById("seed").value, DEFAULT_SEEDS[id]);
  if (seed === null) {
    errorEl.textContent = "Seed must be a non-negative integer (decimal or 0x-prefixed hex).";
    return;
  }
  const lo = seed >>> 0;
  const hi = Math.floor(seed / 0x100000000) >>> 0;
  wasm.exports.run_sample(id, lo, hi);
  const ptr = wasm.exports.frame_ptr();
  const len = wasm.exports.frame_len();
  const bytes = new Uint8Array(wasm.instance.exports.memory.buffer, ptr, len).slice();
  paint(parseFrame(bytes));
}

async function main() {
  const errorEl = document.getElementById("error");
  let instance;
  try {
    const response = await fetch("izanagi_playground_web.wasm");
    if (!response.ok) throw new Error(`HTTP ${response.status}`);
    ({ instance } = await WebAssembly.instantiateStreaming(response, {}));
  } catch (e) {
    errorEl.textContent =
      "Could not load izanagi_playground_web.wasm. Build it first with ./build.sh " +
      "in playground-web/, then serve this directory over HTTP (file:// cannot " +
      "fetch the module). [" + e.message + "]";
    return;
  }
  wasm = { instance, exports: instance.exports };

  const sampleEl = document.getElementById("sample");
  const seedEl = document.getElementById("seed");
  const syncSeed = () => { seedEl.value = "0x" + DEFAULT_SEEDS[Number(sampleEl.value)].toString(16); };
  sampleEl.addEventListener("change", () => { syncSeed(); run(); });
  document.getElementById("run").addEventListener("click", run);
  seedEl.addEventListener("keydown", (ev) => { if (ev.key === "Enter") run(); });
  syncSeed();
  run();
}

main();
