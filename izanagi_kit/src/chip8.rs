//! CHIP-8 virtual machine — the tiny interpreter Joseph Weisbecker
//! wrote for the COSMAC VIP in 1977, today the canonical "first
//! emulator" target. 4 KiB of memory (program at `0x200`, the classic
//! hex font at `0x50`), sixteen `u8` registers `V0`–`VF`, a 16-bit
//! `I`, a 16-deep call stack, a 64×32 XOR-drawn monochrome display,
//! a 4×4 hex keypad, and delay/sound timers ticking at 60 Hz.
//!
//! [`Chip8::step`] executes one opcode; [`Chip8::tick`] decrements the
//! timers (call it once per emulated 60 Hz frame). Everything is
//! deterministic: `CxNN` draws from a seeded [`SplitMix64`], never
//! ambient randomness, and the machine is total — every opcode is
//! defined, malformed programs can only stall, never panic.
//!
//! Ambiguous legacy behavior is behind [`Quirks`]: `Quirks::COSMAC`
//! reproduces the original VIP interpreter (`8xy6`/`8xyE` shift `Vy`,
//! `Fx55`/`Fx65` bump `I`, sprites wrap at edges), `Quirks::MODERN`
//! the common post-1990 conventions.
//!
//! ```
//! use izanagi_kit::chip8::{Chip8, Quirks};
//!
//! // 0x6007 → V0=7; 0x8005 → V0+=5 (carry into VF).
//! let mut c = Chip8::new(1);
//! assert!(c.load_rom(&[0x60, 0x07, 0x70, 0x05]));
//! assert!(c.step());
//! assert_eq!(c.reg(0), 7);
//! c.step();
//! assert_eq!(c.reg(0), 12);
//! ```

use crate::SplitMix64;

/// Display width in pixels.
pub const W: usize = 64;
/// Display height in pixels.
pub const H: usize = 32;
/// Where the built-in hex font lives (conventional address).
pub const FONT_AT: usize = 0x50;
/// Where ROMs load and `pc` starts.
pub const ROM_AT: usize = 0x200;
/// Total memory.
pub const MEM: usize = 4096;
/// Call-stack depth.
pub const STACK: usize = 16;

/// The classic hex-digit font (16 glyphs × 5 bytes).
pub const FONT: [u8; 80] = [
    0xF0, 0x90, 0x90, 0x90, 0xF0, // 0
    0x20, 0x60, 0x20, 0x20, 0x70, // 1
    0xF0, 0x10, 0xF0, 0x80, 0xF0, // 2
    0xF0, 0x10, 0xF0, 0x10, 0xF0, // 3
    0x90, 0x90, 0xF0, 0x10, 0x10, // 4
    0xF0, 0x80, 0xF0, 0x10, 0xF0, // 5
    0xF0, 0x80, 0xF0, 0x90, 0xF0, // 6
    0xF0, 0x10, 0x20, 0x40, 0x40, // 7
    0xF0, 0x90, 0xF0, 0x90, 0xF0, // 8
    0xF0, 0x90, 0xF0, 0x10, 0xF0, // 9
    0xF0, 0x90, 0xF0, 0x90, 0x90, // A
    0xE0, 0x90, 0xE0, 0x90, 0xE0, // B
    0xF0, 0x80, 0x80, 0x80, 0xF0, // C
    0xE0, 0x90, 0x90, 0x90, 0xE0, // D
    0xF0, 0x80, 0xF0, 0x80, 0xF0, // E
    0xF0, 0x80, 0xF0, 0x80, 0x80, // F
];

/// Behavior switches for the ambiguous opcodes (see module docs).
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct Quirks {
    /// `8xy6`/`8xyE`: shift `Vy` (COSMAC) rather than `Vx` (modern).
    pub shift_uses_vy: bool,
    /// `Fx55`/`Fx65`: leave `I` pointing past the block (COSMAC).
    pub load_store_bumps_i: bool,
    /// `Bnnn` adds `Vx` (S-CHIP) rather than `V0` (COSMAC).
    pub jump_uses_x: bool,
    /// `Dxyn` sprite bits wrap around the display edge (COSMAC)
    /// rather than clipping (modern).
    pub sprites_wrap: bool,
}

impl Quirks {
    /// Original COSMAC VIP behavior.
    pub const COSMAC: Self = Quirks {
        shift_uses_vy: true,
        load_store_bumps_i: true,
        jump_uses_x: false,
        sprites_wrap: true,
    };
    /// Common post-1990 (Amiga-era) behavior: the default.
    pub const MODERN: Self = Quirks {
        shift_uses_vy: false,
        load_store_bumps_i: false,
        jump_uses_x: false,
        sprites_wrap: false,
    };
}

/// A CHIP-8 virtual machine. All state is plain data — the machine
/// can be cloned, hashed, and replayed bit-identically.
#[derive(Clone)]
pub struct Chip8 {
    v: [u8; 16],
    i: u16,
    pc: u16,
    stack: [u16; STACK],
    sp: usize,
    mem: [u8; MEM],
    gfx: [u8; W * H],
    keys: [bool; 16],
    delay: u8,
    sound: u8,
    rng: SplitMix64,
    quirks: Quirks,
    wait_key: Option<usize>,
    draw: bool,
}

impl Chip8 {
    /// A fresh machine: font loaded at [`FONT_AT`], `pc` at
    /// [`ROM_AT`], RNG seeded from `seed`.
    pub fn new(seed: u64) -> Self {
        let mut c = Chip8 {
            v: [0; 16],
            i: 0,
            pc: ROM_AT as u16,
            stack: [0; STACK],
            sp: 0,
            mem: [0; MEM],
            gfx: [0; W * H],
            keys: [false; 16],
            delay: 0,
            sound: 0,
            rng: SplitMix64::new(seed),
            quirks: Quirks::MODERN,
            wait_key: None,
            draw: false,
        };
        c.mem[FONT_AT..FONT_AT + FONT.len()].copy_from_slice(&FONT);
        c
    }

    /// Switch the opcode quirks mid-flight (no reset).
    pub fn set_quirks(&mut self, q: Quirks) {
        self.quirks = q;
    }

    /// The active quirk set.
    pub fn quirks(&self) -> Quirks {
        self.quirks
    }

    /// Load a ROM at [`ROM_AT`]. `false` if it doesn't fit in memory.
    pub fn load_rom(&mut self, rom: &[u8]) -> bool {
        if rom.len() > MEM - ROM_AT {
            return false;
        }
        self.mem[ROM_AT..ROM_AT + rom.len()].copy_from_slice(rom);
        self.pc = ROM_AT as u16;
        true
    }

    /// Execute one instruction. `false` when the machine is waiting
    /// for a key (`Fx0A`) — [`Chip8::press`] resumes it.
    pub fn step(&mut self) -> bool {
        if self.wait_key.is_some() {
            return false;
        }
        let at = usize::from(self.pc) % MEM;
        let op = u16::from(self.mem[at]) << 8 | u16::from(self.mem[(at + 1) % MEM]);
        let nnn = op & 0x0FFF;
        let kk = (op & 0xFF) as u8;
        let x = usize::from((op >> 8) & 0xF);
        let y = usize::from((op >> 4) & 0xF);
        let n = usize::from(op & 0xF);
        let mut next = (usize::from(self.pc) + 2) % MEM;
        match op >> 12 {
            0x0 => match op & 0xFF {
                0xE0 => {
                    self.gfx = [0; W * H];
                    self.draw = true;
                }
                0xEE if self.sp > 0 => {
                    self.sp -= 1;
                    next = usize::from(self.stack[self.sp]) % MEM;
                }
                _ => {}
            },
            0x1 => next = nnn as usize,
            0x2 => {
                if self.sp < STACK {
                    self.stack[self.sp] = self.pc + 2;
                    self.sp += 1;
                    next = nnn as usize;
                }
            }
            0x3 => {
                if self.v[x] == kk {
                    next = (next + 2) % MEM;
                }
            }
            0x4 => {
                if self.v[x] != kk {
                    next = (next + 2) % MEM;
                }
            }
            0x5 => {
                if self.v[x] == self.v[y] {
                    next = (next + 2) % MEM;
                }
            }
            0x6 => self.v[x] = kk,
            0x7 => self.v[x] = self.v[x].wrapping_add(kk),
            0x8 => match op & 0xF {
                0x0 => self.v[x] = self.v[y],
                0x1 => self.v[x] |= self.v[y],
                0x2 => self.v[x] &= self.v[y],
                0x3 => self.v[x] ^= self.v[y],
                0x4 => {
                    let s = u16::from(self.v[x]) + u16::from(self.v[y]);
                    self.v[x] = s as u8;
                    self.v[0xF] = u8::from(s > 0xFF);
                }
                0x5 => {
                    let (r, borrow) = self.v[x].overflowing_sub(self.v[y]);
                    self.v[x] = r;
                    self.v[0xF] = u8::from(!borrow);
                }
                0x6 => {
                    let src = if self.quirks.shift_uses_vy {
                        self.v[y]
                    } else {
                        self.v[x]
                    };
                    self.v[x] = src >> 1;
                    self.v[0xF] = src & 1;
                }
                0x7 => {
                    let (r, borrow) = self.v[y].overflowing_sub(self.v[x]);
                    self.v[x] = r;
                    self.v[0xF] = u8::from(!borrow);
                }
                0xE => {
                    let src = if self.quirks.shift_uses_vy {
                        self.v[y]
                    } else {
                        self.v[x]
                    };
                    self.v[x] = src << 1;
                    self.v[0xF] = src >> 7;
                }
                _ => {}
            },
            0x9 => {
                if self.v[x] != self.v[y] {
                    next = (next + 2) % MEM;
                }
            }
            0xA => self.i = nnn,
            0xB => {
                let base = if self.quirks.jump_uses_x {
                    self.v[x]
                } else {
                    self.v[0]
                };
                next = (nnn as usize + usize::from(base)) % MEM;
            }
            0xC => self.v[x] = (self.rng.next_u64() & 0xFF) as u8 & kk,
            0xD => self.draw_sprite(x, y, n),
            0xE => match op & 0xFF {
                0x9E if self.keys[usize::from(self.v[x] & 0xF)] => {
                    next = (next + 2) % MEM;
                }
                0xA1 if !self.keys[usize::from(self.v[x] & 0xF)] => {
                    next = (next + 2) % MEM;
                }
                _ => {}
            },
            0xF => match op & 0xFF {
                0x07 => self.v[x] = self.delay,
                0x0A => self.wait_key = Some(x),
                0x15 => self.delay = self.v[x],
                0x18 => self.sound = self.v[x],
                0x1E => self.i = self.i.wrapping_add(u16::from(self.v[x])),
                0x29 => self.i = (FONT_AT + usize::from(self.v[x] & 0xF) * 5) as u16,
                0x33 => {
                    let at = usize::from(self.i);
                    if at + 3 <= MEM {
                        let val = self.v[x];
                        self.mem[at] = val / 100;
                        self.mem[at + 1] = (val / 10) % 10;
                        self.mem[at + 2] = val % 10;
                    }
                }
                0x55 => {
                    let at = usize::from(self.i);
                    if at + x < MEM {
                        self.mem[at..at + x + 1].copy_from_slice(&self.v[..=x]);
                    }
                    if self.quirks.load_store_bumps_i {
                        self.i = self.i.wrapping_add((x + 1) as u16);
                    }
                }
                0x65 => {
                    let at = usize::from(self.i);
                    if at + x < MEM {
                        self.v[..=x].copy_from_slice(&self.mem[at..at + x + 1]);
                    }
                    if self.quirks.load_store_bumps_i {
                        self.i = self.i.wrapping_add((x + 1) as u16);
                    }
                }
                _ => {}
            },
            _ => {}
        }
        self.pc = next as u16;
        true
    }

    fn draw_sprite(&mut self, x: usize, y: usize, n: usize) {
        let x0 = usize::from(self.v[x]) % W;
        let y0 = usize::from(self.v[y]) % H;
        let mut collision = false;
        for row in 0..n {
            let py = y0 + row;
            if py >= H {
                break;
            }
            let byte = self.mem[(usize::from(self.i) + row) % MEM];
            for col in 0..8 {
                if byte & (0x80 >> col) == 0 {
                    continue;
                }
                let px = x0 + col;
                let idx = if px >= W {
                    if self.quirks.sprites_wrap {
                        py * W + (px % W)
                    } else {
                        break;
                    }
                } else {
                    py * W + px
                };
                if self.gfx[idx] != 0 {
                    collision = true;
                }
                self.gfx[idx] ^= 1;
            }
        }
        self.v[0xF] = u8::from(collision);
        if n > 0 {
            self.draw = true;
        }
    }

    /// Advance the 60 Hz timers once (delay and sound saturate at 0).
    pub fn tick(&mut self) {
        self.delay = self.delay.saturating_sub(1);
        self.sound = self.sound.saturating_sub(1);
    }

    /// Key `k` (0–F) pressed: records the state and completes a
    /// pending `Fx0A` wait.
    pub fn press(&mut self, k: usize) {
        if k >= 16 {
            return;
        }
        self.keys[k] = true;
        if let Some(x) = self.wait_key.take() {
            self.v[x] = k as u8;
            self.pc = (self.pc + 2) % MEM as u16;
        }
    }

    /// Key `k` released.
    pub fn release(&mut self, k: usize) {
        if k < 16 {
            self.keys[k] = false;
        }
    }

    /// Current key state.
    pub fn key(&self, k: usize) -> bool {
        k < 16 && self.keys[k]
    }

    /// `V[x]` (masked to 0–F).
    pub fn reg(&self, x: usize) -> u8 {
        self.v[x & 0xF]
    }

    /// The `I` register.
    pub fn ireg(&self) -> u16 {
        self.i
    }

    /// Program counter.
    pub fn pc(&self) -> u16 {
        self.pc
    }

    /// Stack pointer (number of live return addresses).
    pub fn sp(&self) -> usize {
        self.sp
    }

    /// Delay timer value.
    pub fn delay(&self) -> u8 {
        self.delay
    }

    /// Sound timer value (beep while nonzero).
    pub fn sound(&self) -> u8 {
        self.sound
    }

    /// The register index a pending `Fx0A` writes to, if waiting.
    pub fn waiting_key(&self) -> Option<usize> {
        self.wait_key
    }

    /// Pixel `(x, y)` — `false` out of bounds.
    pub fn pixel(&self, x: usize, y: usize) -> bool {
        x < W && y < H && self.gfx[y * W + x] != 0
    }

    /// The whole 64×32 framebuffer (1 = lit).
    pub fn pixels(&self) -> &[u8] {
        &self.gfx
    }

    /// Whether anything drew since the last [`Chip8::clear_draw_flag`].
    pub fn draw_flag(&self) -> bool {
        self.draw
    }

    /// Clear the draw flag (call once per presented frame).
    pub fn clear_draw_flag(&mut self) {
        self.draw = false;
    }

    /// Read-only view of all 4 KiB of memory.
    pub fn memory(&self) -> &[u8] {
        &self.mem
    }
}

/// The full ROM input any `Chip8` game needs between two frames —
/// for replay/driver code: `steps` instructions, then one timer tick.
pub fn run_frame(c: &mut Chip8, steps: usize, keys_down: &[usize]) {
    for &k in keys_down {
        c.press(k);
    }
    for k in 0..16 {
        if !keys_down.contains(&k) {
            c.release(k);
        }
    }
    for _ in 0..steps {
        c.step();
    }
    c.tick();
}

#[cfg(test)]
mod tests {
    use super::*;

    fn rom(ops: &[u16]) -> Vec<u8> {
        ops.iter().flat_map(|o| o.to_be_bytes()).collect()
    }

    fn boot(ops: &[u16]) -> Chip8 {
        let mut c = Chip8::new(42);
        assert!(c.load_rom(&rom(ops)));
        c
    }

    #[test]
    fn arithmetic_and_carry() {
        // V0=200; V0+=V1(100) wraps to 44 with VF=1; V0-=V1 borrows → VF=0.
        let mut c = boot(&[0x60C8, 0x6164, 0x8014, 0x8015]);
        c.step();
        c.step();
        c.step();
        assert_eq!(c.reg(0), 44);
        assert_eq!(c.reg(0xF), 1);
        c.step();
        assert_eq!(c.reg(0xF), 0);
        assert_eq!(c.reg(0), 44u8.wrapping_sub(100));
    }

    #[test]
    fn call_ret_and_skips() {
        // 0x200: call 0x208 → V0=9, ret → 0x202: skip if V0==9 → pc 0x206
        // (skipping the jump at 0x204 that would re-enter the sub).
        let mut c = boot(&[0x2208, 0x3009, 0x120A, 0x60FF, 0x6009, 0x00EE]);
        c.step();
        assert_eq!(c.pc(), 0x208);
        assert_eq!(c.sp(), 1);
        c.step(); // V0=9
        c.step(); // ret
        assert_eq!(c.pc(), 0x202);
        assert_eq!(c.sp(), 0);
        c.step(); // 3xkk skip → pc += 4
        assert_eq!(c.pc(), 0x206);
        assert_eq!(c.reg(0), 9);
    }

    #[test]
    fn quirks_shift_source() {
        let prog = &[0x6105, 0x6010, 0x8016]; // V1=5 V0=16 V0>>=1
        let mut modern = boot(prog);
        modern.step();
        modern.step();
        modern.step();
        assert_eq!(modern.reg(0), 8);
        let mut cosmac = Chip8::new(42);
        cosmac.set_quirks(Quirks::COSMAC);
        assert!(cosmac.load_rom(&rom(prog)));
        cosmac.step();
        cosmac.step();
        cosmac.step();
        assert_eq!(cosmac.reg(0), 2); // shifted V1=5 >> 1
        assert_eq!(cosmac.quirks(), Quirks::COSMAC);
    }

    #[test]
    fn draw_xor_and_collision() {
        // I=0x300 via A; sprite byte 0xFF at 0x300 drawn twice → collide, VF=1.
        let mut c = boot(&[0xA300, 0x6000, 0x6100, 0xD015, 0xD015]);
        c.mem[0x300] = 0xFF;
        for _ in 0..3 {
            c.step();
        }
        c.step(); // first draw, VF=0
        assert_eq!(c.reg(0xF), 0);
        assert!(c.pixel(0, 0));
        assert!(c.draw_flag());
        c.step(); // second draw erases, VF=1
        assert_eq!(c.reg(0xF), 1);
        assert!(!c.pixel(0, 0));
        assert_eq!(c.pixels().iter().filter(|&&p| p != 0).count(), 0);
        c.clear_draw_flag();
        assert!(!c.draw_flag());
    }

    #[test]
    fn timers_bcd_memory_font() {
        let mut c = boot(&[
            0x607B, // V0=123
            0x6110, // V1=16
            0xF015, // delay=V0
            0xF118, // sound=V1
            0xA300, // I=0x300
            0xF033, // BCD: mem[300..303] = 1,2,3
            0xF265, // V0..V2 = mem[I..I+3] → 1,2,3
            0xF029, // I = FONT_AT + (V0&F)*5 = 0x55
        ]);
        for _ in 0..8 {
            c.step();
        }
        assert_eq!(&c.memory()[0x300..0x303], &[1, 2, 3]);
        assert_eq!((c.reg(0), c.reg(1), c.reg(2)), (1, 2, 3));
        assert_eq!(c.ireg(), 0x55);
        c.tick();
        assert_eq!((c.delay(), c.sound()), (122, 15));
        c.tick();
        assert_eq!(c.delay(), 121);
    }

    #[test]
    fn wait_key_resumes() {
        let mut c = boot(&[0xF00A, 0x6009]);
        c.step();
        assert_eq!(c.waiting_key(), Some(0));
        assert!(!c.step());
        c.press(0xB);
        assert_eq!(c.reg(0), 0xB);
        assert!(c.step());
        assert!(c.key(0xB));
        c.release(0xB);
        assert!(!c.key(0xB));
    }

    #[test]
    fn key_skips_and_frame() {
        // Ex9E skip if key[V0] down; we hold key 9.
        let mut c = boot(&[0x6009, 0xE09E, 0x6101, 0x6102]);
        c.step();
        c.press(9);
        c.step();
        c.step();
        assert_eq!(c.reg(1), 2);
        // run_frame: keys_down keeps 9 pressed, timers tick once.
        let mut c2 = boot(&[0x6009, 0xE09E, 0x6101, 0x6102, 0xF115]);
        run_frame(&mut c2, 5, &[9]);
        assert_eq!(c2.reg(1), 2);
    }
}
