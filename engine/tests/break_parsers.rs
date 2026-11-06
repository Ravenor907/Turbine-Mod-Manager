//! Break tests for everything that reads untrusted input: raw DEFLATE, zip archives, JSON, VDF and the Steam and
//! Heroic scans built on them. A small fixed-seed fuzzer (no outside crates) mutates valid inputs and checks that
//! every call returns Ok or Err: no panic, no hang, no absurd allocation, no file outside the target folder.
//!
//! Tests marked `#[ignore = "DEFECT: ..."]` show a real problem; run them with `-- --ignored` once it is fixed.
//! Some of them crash the whole test process (stack overflow, failed allocation), which is the point.

use std::alloc::{GlobalAlloc, Layout, System};
use std::cell::Cell;
use std::collections::HashMap;
use std::fs;
use std::panic::{catch_unwind, AssertUnwindSafe};
use std::path::{Component, Path, PathBuf};
use std::sync::atomic::{AtomicU64, Ordering};
use std::sync::mpsc::{self, RecvTimeoutError};
use std::time::{Duration, Instant};
use turbine_engine::inflate::{inflate, InflateError};
use turbine_engine::json::{self, Json};
use turbine_engine::vdf::{self, Vdf};
use turbine_engine::zip::{crc32, Crc32, Limits, Zip, ZipError};

// ------------------------------------------------------------------------------------------------------------------
// Allocation watch: the largest single allocation request made by the current thread since the last reset.

struct Watch;

thread_local! {
    static PEAK: Cell<usize> = const { Cell::new(0) };
}

fn note(size: usize) {
    let _ = PEAK.try_with(|p| {
        if size > p.get() {
            p.set(size);
        }
    });
}

unsafe impl GlobalAlloc for Watch {
    unsafe fn alloc(&self, l: Layout) -> *mut u8 {
        note(l.size());
        unsafe { System.alloc(l) }
    }
    unsafe fn alloc_zeroed(&self, l: Layout) -> *mut u8 {
        note(l.size());
        unsafe { System.alloc_zeroed(l) }
    }
    unsafe fn dealloc(&self, p: *mut u8, l: Layout) {
        unsafe { System.dealloc(p, l) }
    }
    unsafe fn realloc(&self, p: *mut u8, l: Layout, new_size: usize) -> *mut u8 {
        note(new_size);
        unsafe { System.realloc(p, l, new_size) }
    }
}

#[global_allocator]
static GLOBAL: Watch = Watch;

fn reset_peak() {
    PEAK.with(|p| p.set(0));
}

fn peak() -> usize {
    PEAK.with(Cell::get)
}

/// Any single allocation above this, while handling one fuzz input, counts as absurd.
const ABSURD: usize = 256 << 20;

// ------------------------------------------------------------------------------------------------------------------
// Fixed-seed PRNG (xorshift64*), so every failure names a seed and an iteration that reproduce it.

struct Rng(u64);

impl Rng {
    fn new(seed: u64) -> Self {
        Rng((seed ^ 0x9E37_79B9_7F4A_7C15) | 1)
    }
    fn next(&mut self) -> u64 {
        let mut x = self.0;
        x ^= x >> 12;
        x ^= x << 25;
        x ^= x >> 27;
        self.0 = x;
        x.wrapping_mul(0x2545_F491_4F6C_DD1D)
    }
    fn below(&mut self, n: usize) -> usize {
        if n == 0 {
            0
        } else {
            (self.next() % n as u64) as usize
        }
    }
    fn chance(&mut self, percent: u64) -> bool {
        self.next() % 100 < percent
    }
    fn bytes(&mut self, n: usize) -> Vec<u8> {
        (0..n).map(|_| self.next() as u8).collect()
    }
    fn pick<'a, T>(&mut self, items: &'a [T]) -> &'a T {
        &items[self.below(items.len())]
    }
}

const INTERESTING: [u64; 16] = [
    0,
    1,
    0x7F,
    0x80,
    0xFF,
    0x7FFF,
    0x8000,
    0xFFFF,
    0x1_0000,
    0x7FFF_FFFF,
    0x8000_0000,
    0xFFFF_FFFE,
    0xFFFF_FFFF,
    0x1_0000_0000,
    i64::MAX as u64,
    u64::MAX,
];

fn put_le(data: &mut [u8], at: usize, v: u64, width: usize) {
    for k in 0..width {
        if let Some(b) = data.get_mut(at + k) {
            *b = (v >> (8 * k)) as u8;
        }
    }
}

/// One random damage: bit flips, random bytes, truncation, duplicated or deleted chunks, inserted noise, huge
/// length fields. With `zip_aware`, also hits fields of zip headers directly.
fn mutate_once(rng: &mut Rng, data: &mut Vec<u8>, zip_aware: bool) {
    if data.is_empty() {
        let k = 1 + rng.below(16);
        data.extend(rng.bytes(k));
        return;
    }
    let len = data.len();
    match rng.below(if zip_aware { 9 } else { 8 }) {
        0 => {
            let i = rng.below(len);
            data[i] ^= 1 << rng.below(8);
        }
        1 => {
            let i = rng.below(len);
            data[i] = rng.next() as u8;
        }
        2 => data.truncate(rng.below(len)),
        3 => {
            let a = rng.below(len);
            let n = 1 + rng.below((len - a).min(512));
            let chunk = data[a..a + n].to_vec();
            let at = rng.below(len + 1);
            data.splice(at..at, chunk);
        }
        4 => {
            let a = rng.below(len);
            let n = 1 + rng.below((len - a).min(64));
            data.drain(a..a + n);
        }
        5 => {
            let at = rng.below(len + 1);
            let n = 1 + rng.below(32);
            let noise = rng.bytes(n);
            data.splice(at..at, noise);
        }
        6 | 7 => {
            let width = *rng.pick(&[1usize, 2, 4, 8]);
            let v = *rng.pick(&INTERESTING);
            let at = rng.below(len);
            put_le(data, at, v, width);
        }
        _ => {
            // Zip header field: find the PK signatures and overwrite one field with a huge or odd value.
            let sigs: Vec<usize> = (0..len.saturating_sub(3))
                .filter(|&i| data[i] == b'P' && data[i + 1] == b'K' && matches!((data[i + 2], data[i + 3]), (1, 2) | (3, 4) | (5, 6) | (6, 6) | (6, 7)))
                .collect();
            if sigs.is_empty() {
                let i = rng.below(len);
                data[i] = rng.next() as u8;
                return;
            }
            let at = *rng.pick(&sigs) + 4 + rng.below(52);
            let width = *rng.pick(&[2usize, 4, 8]);
            let v = *rng.pick(&INTERESTING);
            put_le(data, at, v, width);
        }
    }
}

fn mutate(rng: &mut Rng, seed: &[u8], zip_aware: bool) -> Vec<u8> {
    let mut d = seed.to_vec();
    for _ in 0..1 + rng.below(if zip_aware { 2 } else { 4 }) {
        mutate_once(rng, &mut d, zip_aware);
    }
    d
}

// ------------------------------------------------------------------------------------------------------------------
// Harness: every campaign runs on its own thread with a deadline (a hang fails the test instead of stalling it),
// and every iteration is wrapped so a panic names the seed and iteration.

fn guarded<T: Send + 'static>(what: &'static str, budget: Duration, f: impl FnOnce() -> T + Send + 'static) -> T {
    let (tx, rx) = mpsc::channel();
    let h = std::thread::Builder::new()
        .name(what.into())
        .spawn(move || {
            let _ = tx.send(f());
        })
        .unwrap();
    match rx.recv_timeout(budget) {
        Ok(v) => {
            let _ = h.join();
            v
        }
        Err(RecvTimeoutError::Timeout) => panic!("{what}: still running after {budget:?}, so something hangs"),
        Err(RecvTimeoutError::Disconnected) => match h.join() {
            Err(p) => std::panic::resume_unwind(p),
            Ok(()) => unreachable!(),
        },
    }
}

/// Runs one fuzz case: fails on a panic, a call slower than `slow`, or an absurd allocation.
fn case<T>(what: &str, seed: u64, i: usize, input: &[u8], slow: Duration, f: impl FnOnce() -> T) -> T {
    reset_peak();
    let t = Instant::now();
    let r = catch_unwind(AssertUnwindSafe(f));
    let took = t.elapsed();
    let big = peak();
    let keep = |why: &str| {
        let p = std::env::temp_dir().join(format!("turbine-break-{what}-{seed:x}-{i}.bin"));
        let _ = fs::write(&p, input);
        panic!("{what}: seed {seed:#x} iteration {i}: {why} (input saved to {})", p.display());
    };
    match r {
        Err(_) => keep("panicked"),
        Ok(v) => {
            if took > slow {
                keep(&format!("took {took:?}"));
            }
            if big > ABSURD {
                keep(&format!("asked for {big} bytes at once"));
            }
            v
        }
    }
}

static ITERATIONS: AtomicU64 = AtomicU64::new(0);

fn counted(n: u64) {
    ITERATIONS.fetch_add(n, Ordering::Relaxed);
}

fn scratch(tag: &str) -> PathBuf {
    let p = std::env::temp_dir().join(format!("turbine-break-{tag}-{}", std::process::id()));
    let _ = fs::remove_dir_all(&p);
    fs::create_dir_all(&p).unwrap();
    p
}

// ------------------------------------------------------------------------------------------------------------------
// A tiny DEFLATE writer of our own (stored blocks, and fixed Huffman with greedy LZ77), used to build valid inputs.

struct BitW {
    out: Vec<u8>,
    acc: u64,
    n: u32,
}

impl BitW {
    fn new() -> Self {
        BitW { out: Vec::new(), acc: 0, n: 0 }
    }
    fn put(&mut self, v: u32, bits: u32) {
        self.acc |= (v as u64) << self.n;
        self.n += bits;
        while self.n >= 8 {
            self.out.push(self.acc as u8);
            self.acc >>= 8;
            self.n -= 8;
        }
    }
    /// Huffman codes go most significant bit first.
    fn code(&mut self, code: u32, len: u32) {
        let mut r = 0;
        for k in 0..len {
            r |= ((code >> k) & 1) << (len - 1 - k);
        }
        self.put(r, len);
    }
    fn flush(&mut self) {
        if self.n > 0 {
            self.out.push(self.acc as u8);
            self.acc = 0;
            self.n = 0;
        }
    }
    fn finish(mut self) -> Vec<u8> {
        self.flush();
        self.out
    }
}

const LEN_BASE: [u32; 29] = [3, 4, 5, 6, 7, 8, 9, 10, 11, 13, 15, 17, 19, 23, 27, 31, 35, 43, 51, 59, 67, 83, 99, 115, 131, 163, 195, 227, 258];
const LEN_EXTRA: [u32; 29] = [0, 0, 0, 0, 0, 0, 0, 0, 1, 1, 1, 1, 2, 2, 2, 2, 3, 3, 3, 3, 4, 4, 4, 4, 5, 5, 5, 5, 0];
const DIST_BASE: [u32; 30] = [
    1, 2, 3, 4, 5, 7, 9, 13, 17, 25, 33, 49, 65, 97, 129, 193, 257, 385, 513, 769, 1025, 1537, 2049, 3073, 4097, 6145, 8193, 12289,
    16385, 24577,
];
const DIST_EXTRA: [u32; 30] = [0, 0, 0, 0, 1, 1, 2, 2, 3, 3, 4, 4, 5, 5, 6, 6, 7, 7, 8, 8, 9, 9, 10, 10, 11, 11, 12, 12, 13, 13];

fn fixed_lit(w: &mut BitW, sym: u32) {
    match sym {
        0..=143 => w.code(0x30 + sym, 8),
        144..=255 => w.code(0x190 + sym - 144, 9),
        256..=279 => w.code(sym - 256, 7),
        _ => w.code(0xC0 + sym - 280, 8),
    }
}

fn stored_blocks(w: &mut BitW, data: &[u8], last: bool) {
    let chunks: Vec<&[u8]> = if data.is_empty() { vec![&[][..]] } else { data.chunks(65_535).collect() };
    let n = chunks.len();
    for (k, c) in chunks.into_iter().enumerate() {
        w.put(u32::from(last && k + 1 == n), 1);
        w.put(0, 2);
        w.flush();
        let l = c.len() as u16;
        w.out.extend_from_slice(&l.to_le_bytes());
        w.out.extend_from_slice(&(!l).to_le_bytes());
        w.out.extend_from_slice(c);
    }
}

fn fixed_block(w: &mut BitW, data: &[u8], last: bool) {
    w.put(u32::from(last), 1);
    w.put(1, 2);
    let mut seen: HashMap<[u8; 3], usize> = HashMap::new();
    let mut i = 0;
    while i < data.len() {
        let mut best = (0usize, 0usize);
        if i + 3 <= data.len() {
            let key = [data[i], data[i + 1], data[i + 2]];
            if let Some(&p) = seen.get(&key) {
                if i - p <= 32_768 {
                    let mut l = 0;
                    while l < 258 && i + l < data.len() && data[p + l] == data[i + l] {
                        l += 1;
                    }
                    best = (l, i - p);
                }
            }
            seen.insert(key, i);
        }
        if best.0 >= 3 {
            let (len, dist) = (best.0 as u32, best.1 as u32);
            let li = if len == 258 { 28 } else { (0..28).rev().find(|&k| LEN_BASE[k] <= len).unwrap() };
            fixed_lit(w, 257 + li as u32);
            w.put(len - LEN_BASE[li], LEN_EXTRA[li]);
            let di = (0..30).rev().find(|&k| DIST_BASE[k] <= dist).unwrap();
            w.code(di as u32, 5);
            w.put(dist - DIST_BASE[di], DIST_EXTRA[di]);
            for k in i + 1..i + best.0 {
                if k + 3 <= data.len() {
                    seen.insert([data[k], data[k + 1], data[k + 2]], k);
                }
            }
            i += best.0;
        } else {
            fixed_lit(w, u32::from(data[i]));
            i += 1;
        }
    }
    fixed_lit(w, 256);
}

#[derive(Clone, Copy, Debug)]
enum Comp {
    Store,
    StoredBlocks,
    Fixed,
    Mixed,
}

fn deflate(data: &[u8], how: Comp) -> Vec<u8> {
    let mut w = BitW::new();
    match how {
        Comp::Store => return data.to_vec(),
        Comp::StoredBlocks => stored_blocks(&mut w, data, true),
        Comp::Fixed => fixed_block(&mut w, data, true),
        Comp::Mixed => {
            let mid = data.len() / 2;
            stored_blocks(&mut w, &data[..mid], false);
            fixed_block(&mut w, &data[mid..], false);
            fixed_block(&mut w, &[], true);
        }
    }
    w.finish()
}

/// Something that compresses: text with repeats, runs, and some noise.
fn sample(rng: &mut Rng, max: usize) -> Vec<u8> {
    let n = rng.below(max + 1);
    let words: [&[u8]; 6] = [b"Skyrim ", b"Data/meshes/", b"\r\n", b"plugin.esp ", b"\0\0\0\0", b"BepInEx "];
    let mut out = Vec::with_capacity(n);
    while out.len() < n {
        match rng.below(4) {
            0 => out.extend_from_slice(rng.pick(&words)),
            1 => {
                let k = 1 + rng.below(8);
                out.extend(rng.bytes(k));
            }
            2 => {
                let b = rng.next() as u8;
                out.extend(std::iter::repeat_n(b, 1 + rng.below(300)));
            }
            _ => {
                if !out.is_empty() {
                    let a = rng.below(out.len());
                    let l = (1 + rng.below(40)).min(out.len() - a);
                    let c = out[a..a + l].to_vec();
                    out.extend(c);
                }
            }
        }
    }
    out.truncate(n);
    out
}

// ------------------------------------------------------------------------------------------------------------------
// A tiny zip writer of our own. Records are kept separate so hand-made cases can lie in any field.

#[derive(Clone, Debug)]
struct Rec {
    name: Vec<u8>,
    method: u16,
    flags: u16,
    crc: u32,
    csize: u64,
    size: u64,
    made_by: u16,
    ext_attr: u32,
    offset: u64,
}

struct File {
    name: &'static str,
    data: Vec<u8>,
    comp: Comp,
}

fn f(name: &'static str, data: &[u8], comp: Comp) -> File {
    File { name, data: data.to_vec(), comp }
}

/// Local headers and data. Returns the body and one record per file.
fn locals(files: &[File]) -> (Vec<u8>, Vec<Rec>) {
    let mut body = Vec::new();
    let mut recs = Vec::new();
    for file in files {
        let payload = deflate(&file.data, file.comp);
        let rec = Rec {
            name: file.name.as_bytes().to_vec(),
            method: if matches!(file.comp, Comp::Store) { 0 } else { 8 },
            flags: 0x0800,
            crc: crc32(&file.data),
            csize: payload.len() as u64,
            size: file.data.len() as u64,
            made_by: 3 << 8 | 20,
            ext_attr: 0o100644 << 16,
            offset: body.len() as u64,
        };
        local_header(&mut body, &rec, &rec.name);
        body.extend_from_slice(&payload);
        recs.push(rec);
    }
    (body, recs)
}

fn local_header(body: &mut Vec<u8>, r: &Rec, name: &[u8]) {
    body.extend_from_slice(&0x0403_4b50u32.to_le_bytes());
    body.extend_from_slice(&20u16.to_le_bytes());
    body.extend_from_slice(&r.flags.to_le_bytes());
    body.extend_from_slice(&r.method.to_le_bytes());
    body.extend_from_slice(&[0, 0, 0x21, 0]); // time, date
    body.extend_from_slice(&r.crc.to_le_bytes());
    body.extend_from_slice(&(r.csize.min(0xFFFF_FFFF) as u32).to_le_bytes());
    body.extend_from_slice(&(r.size.min(0xFFFF_FFFF) as u32).to_le_bytes());
    body.extend_from_slice(&(name.len() as u16).to_le_bytes());
    body.extend_from_slice(&0u16.to_le_bytes());
    body.extend_from_slice(name);
}

/// Appends the central directory and end records. `zip64` writes every size and offset through the ZIP64 extra field.
fn assemble(mut body: Vec<u8>, recs: &[Rec], zip64: bool) -> Vec<u8> {
    let cd_off = body.len() as u64;
    for r in recs {
        let mut extra = Vec::new();
        let (cs, us, off) = if zip64 {
            extra.extend_from_slice(&1u16.to_le_bytes());
            extra.extend_from_slice(&24u16.to_le_bytes());
            extra.extend_from_slice(&r.size.to_le_bytes());
            extra.extend_from_slice(&r.csize.to_le_bytes());
            extra.extend_from_slice(&r.offset.to_le_bytes());
            (0xFFFF_FFFFu32, 0xFFFF_FFFFu32, 0xFFFF_FFFFu32)
        } else {
            (r.csize as u32, r.size as u32, r.offset as u32)
        };
        body.extend_from_slice(&0x0201_4b50u32.to_le_bytes());
        body.extend_from_slice(&r.made_by.to_le_bytes());
        body.extend_from_slice(&45u16.to_le_bytes());
        body.extend_from_slice(&r.flags.to_le_bytes());
        body.extend_from_slice(&r.method.to_le_bytes());
        body.extend_from_slice(&[0, 0, 0x21, 0]);
        body.extend_from_slice(&r.crc.to_le_bytes());
        body.extend_from_slice(&cs.to_le_bytes());
        body.extend_from_slice(&us.to_le_bytes());
        body.extend_from_slice(&(r.name.len() as u16).to_le_bytes());
        body.extend_from_slice(&(extra.len() as u16).to_le_bytes());
        body.extend_from_slice(&0u16.to_le_bytes()); // comment
        body.extend_from_slice(&0u16.to_le_bytes()); // disk
        body.extend_from_slice(&0u16.to_le_bytes()); // internal attributes
        body.extend_from_slice(&r.ext_attr.to_le_bytes());
        body.extend_from_slice(&off.to_le_bytes());
        body.extend_from_slice(&r.name);
        body.extend_from_slice(&extra);
    }
    let cd_size = body.len() as u64 - cd_off;
    let n = recs.len() as u64;
    if zip64 {
        let rec_off = body.len() as u64;
        body.extend_from_slice(&0x0606_4b50u32.to_le_bytes());
        body.extend_from_slice(&44u64.to_le_bytes());
        body.extend_from_slice(&45u16.to_le_bytes());
        body.extend_from_slice(&45u16.to_le_bytes());
        body.extend_from_slice(&0u32.to_le_bytes());
        body.extend_from_slice(&0u32.to_le_bytes());
        body.extend_from_slice(&n.to_le_bytes());
        body.extend_from_slice(&n.to_le_bytes());
        body.extend_from_slice(&cd_size.to_le_bytes());
        body.extend_from_slice(&cd_off.to_le_bytes());
        body.extend_from_slice(&0x0706_4b50u32.to_le_bytes());
        body.extend_from_slice(&0u32.to_le_bytes());
        body.extend_from_slice(&rec_off.to_le_bytes());
        body.extend_from_slice(&1u32.to_le_bytes());
    }
    body.extend_from_slice(&0x0605_4b50u32.to_le_bytes());
    body.extend_from_slice(&[0, 0, 0, 0]);
    let n16 = if zip64 { 0xFFFF } else { n as u16 };
    body.extend_from_slice(&n16.to_le_bytes());
    body.extend_from_slice(&n16.to_le_bytes());
    let (s32, o32) = if zip64 { (0xFFFF_FFFFu32, 0xFFFF_FFFFu32) } else { (cd_size as u32, cd_off as u32) };
    body.extend_from_slice(&s32.to_le_bytes());
    body.extend_from_slice(&o32.to_le_bytes());
    body.extend_from_slice(&0u16.to_le_bytes());
    body
}

/// Rewrites the local headers so they carry each record's current name, method and flags (for tests that change
/// them after `locals`), moving the data along and updating the offsets.
fn relocal(body: &[u8], recs: &mut [Rec]) -> Vec<u8> {
    let mut out = Vec::new();
    for r in recs.iter_mut() {
        let o = r.offset as usize;
        let skip = 30 + u16::from_le_bytes([body[o + 26], body[o + 27]]) as usize + u16::from_le_bytes([body[o + 28], body[o + 29]]) as usize;
        let data = &body[o + skip..o + skip + r.csize as usize];
        r.offset = out.len() as u64;
        let name = r.name.clone();
        local_header(&mut out, r, &name);
        out.extend_from_slice(data);
    }
    out
}

fn zip_of(files: &[File], zip64: bool) -> Vec<u8> {
    let (body, recs) = locals(files);
    assemble(body, &recs, zip64)
}

fn data_file(name: &str) -> PathBuf {
    Path::new(env!("CARGO_MANIFEST_DIR")).join("tests/data").join(name)
}

/// Raw deflate streams pulled out of the Python-made test archives (dynamic Huffman blocks from zlib).
fn zlib_streams() -> Vec<(Vec<u8>, usize)> {
    let mut out = Vec::new();
    for name in ["good.zip", "bomb.zip", "zip64.zip"] {
        let b = fs::read(data_file(name)).unwrap();
        let mut p = 0;
        while p + 30 <= b.len() && b[p..p + 4] == [0x50, 0x4b, 3, 4] {
            let method = u16::from_le_bytes([b[p + 8], b[p + 9]]);
            let mut cs = u32::from_le_bytes([b[p + 18], b[p + 19], b[p + 20], b[p + 21]]) as usize;
            let mut us = u32::from_le_bytes([b[p + 22], b[p + 23], b[p + 24], b[p + 25]]) as usize;
            let nl = u16::from_le_bytes([b[p + 26], b[p + 27]]) as usize;
            let xl = u16::from_le_bytes([b[p + 28], b[p + 29]]) as usize;
            let extra = &b[p + 30 + nl..p + 30 + nl + xl];
            if cs == 0xFFFF_FFFF && extra.len() >= 20 {
                us = u64::from_le_bytes(extra[4..12].try_into().unwrap()) as usize;
                cs = u64::from_le_bytes(extra[12..20].try_into().unwrap()) as usize;
            }
            let start = p + 30 + nl + xl;
            if method == 8 && start + cs <= b.len() && us < (4 << 20) {
                out.push((b[start..start + cs].to_vec(), us));
            }
            p = start + cs;
        }
    }
    out
}

/// Small valid archives covering every path the reader has.
fn zip_seeds(rng: &mut Rng) -> Vec<Vec<u8>> {
    let text = sample(rng, 3000);
    let mut link = locals(&[f("real.txt", b"real", Comp::Store), f("link", b"real.txt", Comp::Store)]);
    link.1[1].ext_attr = 0o120777 << 16;
    let mut cp437 = locals(&[f("x", b"cp", Comp::Store)]);
    cp437.1[0].name = vec![b'a', 0x81, 0xE1, b'.', b't'];
    cp437.1[0].flags = 0;
    cp437.0 = relocal(&cp437.0, &mut cp437.1);
    let mut seeds = vec![
        zip_of(&[f("a.txt", b"hello", Comp::Store)], false),
        zip_of(&[f("Data/", b"", Comp::Store), f("Data/x.esp", &text, Comp::Fixed), f("Data/y.txt", &text, Comp::Mixed)], false),
        zip_of(&[f("BepInEx/plugins/m.dll", &text, Comp::StoredBlocks), f("empty", b"", Comp::Fixed)], false),
        zip_of(&[f("z/a", &text, Comp::Fixed), f("z/b", b"tiny", Comp::Store)], true),
        assemble(link.0, &link.1, false),
        assemble(cp437.0, &cp437.1, false),
    ];
    for name in ["good.zip", "zip64.zip", "link.zip", "evil.zip"] {
        seeds.push(fs::read(data_file(name)).unwrap());
    }
    seeds
}

// ------------------------------------------------------------------------------------------------------------------
// Checks on what extract_all leaves on disk.

fn walk(dir: &Path, out: &mut Vec<PathBuf>) {
    let Ok(rd) = fs::read_dir(dir) else { return };
    for e in rd.flatten() {
        let p = e.path();
        out.push(p.clone());
        if e.file_type().map(|t| t.is_dir()).unwrap_or(false) {
            walk(&p, out);
        }
    }
}

/// After any extract_all into `parent/dest`: only `dest` may exist in `parent`, nothing in it is a link, every
/// returned path is a plain relative path that exists inside it, and on failure nothing at all is left.
fn check_extract(parent: &Path, dest: &Path, r: &Result<Vec<PathBuf>, ZipError>) -> Result<(), String> {
    let left: Vec<String> = fs::read_dir(parent).unwrap().flatten().map(|e| e.file_name().to_string_lossy().into_owned()).collect();
    match r {
        Ok(written) => {
            if left != ["out"] {
                return Err(format!("after success the folder holds {left:?}"));
            }
            for w in written {
                if !w.components().all(|c| matches!(c, Component::Normal(_))) {
                    return Err(format!("returned path {w:?} is not plain"));
                }
                if !dest.join(w).is_file() {
                    return Err(format!("returned path {w:?} doesn't exist"));
                }
            }
            let mut all = Vec::new();
            walk(dest, &mut all);
            for p in all {
                let m = fs::symlink_metadata(&p).unwrap();
                if m.file_type().is_symlink() {
                    return Err(format!("{p:?} is a link"));
                }
                if !p.starts_with(dest) {
                    return Err(format!("{p:?} is outside"));
                }
            }
        }
        Err(_) => {
            if !left.is_empty() {
                return Err(format!("after a failure the folder still holds {left:?}"));
            }
        }
    }
    Ok(())
}

// ------------------------------------------------------------------------------------------------------------------
// Fuzz: raw DEFLATE.

#[test]
fn fuzz_inflate() {
    let n = guarded("fuzz_inflate", Duration::from_secs(40), || {
        let mut runs = 0u64;
        let slow = Duration::from_secs(2);
        // 1. Our own encoder: every valid stream must come back exactly, and stop at the size limit.
        let seed = 0x1F1A_7E01;
        let mut rng = Rng::new(seed);
        let mut valid = Vec::new();
        for i in 0..1500 {
            let data = sample(&mut rng, 5000);
            let how = *rng.pick(&[Comp::StoredBlocks, Comp::Fixed, Comp::Mixed]);
            let enc = deflate(&data, how);
            let got = case("inflate-valid", seed, i, &enc, slow, || inflate(&enc, data.len()));
            assert_eq!(got.as_deref().ok(), Some(&data[..]), "{how:?} round trip, iteration {i}");
            if !data.is_empty() {
                let short = case("inflate-limit", seed, i, &enc, slow, || inflate(&enc, data.len() - 1));
                assert_eq!(short, Err(InflateError::TooLarge), "limit not enforced, iteration {i}");
            }
            valid.push(enc);
            runs += 2;
        }
        // 2. Damaged versions of those, plus zlib's own dynamic-Huffman streams.
        let mut seeds: Vec<(Vec<u8>, usize)> = valid.into_iter().map(|v| (v, 1 << 16)).collect();
        seeds.extend(zlib_streams());
        let seed = 0x1F1A_7E02;
        let mut rng = Rng::new(seed);
        for i in 0..30_000 {
            let (s, lim) = &seeds[rng.below(seeds.len())];
            let m = mutate(&mut rng, s, false);
            let limit = if rng.chance(20) { rng.below(*lim + 1) } else { *lim };
            let r = case("inflate-mutated", seed, i, &m, slow, || inflate(&m, limit));
            if let Ok(out) = r {
                assert!(out.len() <= limit, "output {} over limit {limit}, iteration {i}", out.len());
            }
            runs += 1;
        }
        // 3. Random bytes, mostly forced to start a dynamic block so the table-building code gets hammered.
        let seed = 0x1F1A_7E03;
        let mut rng = Rng::new(seed);
        for i in 0..40_000 {
            let k = 1 + rng.below(200);
            let mut m = rng.bytes(k);
            if rng.chance(80) {
                m[0] = (m[0] & !0b111) | if rng.chance(50) { 0b101 } else { 0b100 };
            }
            let r = case("inflate-random", seed, i, &m, slow, || inflate(&m, 1 << 20));
            if let Ok(out) = r {
                assert!(out.len() <= 1 << 20);
            }
            runs += 1;
        }
        runs
    });
    counted(n);
    eprintln!("fuzz_inflate: {n} runs clean");
}

#[test]
fn inflate_hand_made() {
    // Empty input, a lone header bit, reserved block type 3.
    assert_eq!(inflate(&[], 10), Err(InflateError::Truncated));
    assert_eq!(inflate(&[1], 10), Err(InflateError::Truncated));
    assert_eq!(inflate(&[0b111], 10), Err(InflateError::BadBlock));
    // Stored block with LEN and NLEN that disagree, and one that claims more than is there.
    assert_eq!(inflate(&[1, 5, 0, 0, 0, b'a'], 10), Err(InflateError::BadBlock));
    assert_eq!(inflate(&[1, 5, 0, 0xFA, 0xFF, b'a'], 10), Err(InflateError::Truncated));
    // Fixed block whose first code is a back-reference (nothing to copy from).
    let mut w = BitW::new();
    w.put(1, 1);
    w.put(1, 2);
    fixed_lit(&mut w, 257);
    w.code(0, 5);
    fixed_lit(&mut w, 256);
    assert_eq!(inflate(&w.finish(), 10), Err(InflateError::BadDistance));
    // Fixed block using the two length codes and two distance codes that don't exist.
    for (lit, dist) in [(286u32, 0u32), (287, 0), (257, 30), (257, 31)] {
        let mut w = BitW::new();
        w.put(1, 1);
        w.put(1, 2);
        fixed_lit(&mut w, b'a' as u32);
        fixed_lit(&mut w, lit);
        w.code(dist, 5);
        fixed_lit(&mut w, 256);
        assert!(inflate(&w.finish(), 1000).is_err(), "{lit} {dist}");
    }
    // A run of back-references far past the limit stops at the limit, quickly, without allocating the whole lot.
    let mut w = BitW::new();
    w.put(1, 1);
    w.put(1, 2);
    fixed_lit(&mut w, 0);
    for _ in 0..100_000 {
        fixed_lit(&mut w, 285);
        w.code(0, 5);
    }
    fixed_lit(&mut w, 256);
    let enc = w.finish();
    reset_peak();
    let t = Instant::now();
    assert_eq!(inflate(&enc, 1 << 20), Err(InflateError::TooLarge));
    assert!(t.elapsed() < Duration::from_secs(2));
    assert!(peak() <= 16 << 20, "asked for {} bytes", peak());
    // A huge limit must not be allocated up front.
    reset_peak();
    assert!(inflate(&deflate(b"abc", Comp::Fixed), usize::MAX).is_ok());
    assert!(peak() <= 16 << 20, "asked for {} bytes", peak());
    // Limit 0 accepts only empty output.
    assert_eq!(inflate(&deflate(b"", Comp::Fixed), 0), Ok(vec![]));
    assert_eq!(inflate(&deflate(b"a", Comp::StoredBlocks), 0), Err(InflateError::TooLarge));
}

// ------------------------------------------------------------------------------------------------------------------
// Fuzz: zip reading and unpacking.

/// Fixed defect D1 (see `zip_huge_compressed_size_crashes`): open() must refuse any entry whose data runs past the
/// end of the file, so no read can be asked to allocate more than the file holds.
fn assert_sizes_fit(z: &Zip, file_len: u64, i: usize) {
    for e in &z.entries {
        assert!(e.compressed <= file_len, "{:?} claims {} compressed bytes in a {file_len}-byte file, iteration {i}", e.name, e.compressed);
    }
}

#[test]
fn fuzz_zip_open_and_read() {
    let n = guarded("fuzz_zip_open_and_read", Duration::from_secs(45), || {
        let dir = scratch("zipread");
        let path = dir.join("in.zip");
        let seed = 0x21B0_0001;
        let mut rng = Rng::new(seed);
        let seeds = zip_seeds(&mut rng);
        let limits = Limits { total: 32 << 20, ratio: 400 };
        let slow = Duration::from_secs(2);
        let (mut runs, mut opened, mut read_ok) = (0u64, 0u64, 0u64);
        for i in 0..40_000 {
            let s = &seeds[rng.below(seeds.len())];
            let m = mutate(&mut rng, s, true);
            fs::write(&path, &m).unwrap();
            let opened_zip = case("zip-open", seed, i, &m, slow, || Zip::open(&path));
            runs += 1;
            let Ok(mut z) = opened_zip else { continue };
            opened += 1;
            for e in &z.entries {
                assert!(
                    Path::new(&e.name).components().all(|c| matches!(c, Component::Normal(_))),
                    "unsafe name {:?} accepted, iteration {i}",
                    e.name
                );
            }
            assert_sizes_fit(&z, m.len() as u64, i);
            let entries = z.entries.clone();
            for e in entries.iter().take(64) {
                let r = case("zip-read", seed, i, &m, slow, || z.read(e, &limits));
                runs += 1;
                if let Ok(out) = r {
                    read_ok += 1;
                    assert_eq!(crc32(&out), crc32(&out)); // touch the data
                    assert_eq!(out.len() as u64, e.size, "read gave a size other than declared, iteration {i}");
                }
            }
        }
        let _ = fs::remove_dir_all(&dir);
        eprintln!("fuzz_zip_open_and_read: {runs} runs clean, {opened} opened, {read_ok} reads ok");
        runs
    });
    counted(n);
}

#[test]
fn fuzz_zip_extract_all() {
    let n = guarded("fuzz_zip_extract_all", Duration::from_secs(45), || {
        let dir = scratch("zipextract");
        let input = dir.join("in.zip");
        let parent = dir.join("games");
        let dest = parent.join("out");
        let seed = 0x21B0_0002;
        let mut rng = Rng::new(seed);
        // Only the small seeds: extraction writes and syncs real files.
        let seeds: Vec<Vec<u8>> = zip_seeds(&mut rng).into_iter().filter(|s| s.len() < 8000).collect();
        let limits = Limits { total: 32 << 20, ratio: 400 };
        let slow = Duration::from_secs(5);
        let (mut runs, mut ok) = (0u64, 0u64);
        for i in 0..15_000 {
            let s = &seeds[rng.below(seeds.len())];
            let m = if i < seeds.len() { s.clone() } else { mutate(&mut rng, s, true) };
            fs::write(&input, &m).unwrap();
            let _ = fs::remove_dir_all(&parent);
            fs::create_dir_all(&parent).unwrap();
            let Ok(mut z) = Zip::open(&input) else { continue };
            assert_sizes_fit(&z, m.len() as u64, i);
            let r = case("zip-extract", seed, i, &m, slow, || z.extract_all(&dest, &limits));
            runs += 1;
            if let Err(why) = check_extract(&parent, &dest, &r) {
                let p = std::env::temp_dir().join(format!("turbine-break-extract-{seed:x}-{i}.zip"));
                let _ = fs::write(&p, &m);
                panic!("extract_all, seed {seed:#x} iteration {i}: {why} ({r:?}), input saved to {}", p.display());
            }
            if r.is_ok() {
                ok += 1;
            }
        }
        // Nothing may ever appear next to the games folder either.
        let mut around: Vec<String> = fs::read_dir(&dir).unwrap().flatten().map(|e| e.file_name().to_string_lossy().into_owned()).collect();
        around.sort();
        assert_eq!(around, ["games", "in.zip"]);
        let _ = fs::remove_dir_all(&dir);
        eprintln!("fuzz_zip_extract_all: {runs} runs clean, {ok} unpacked");
        runs
    });
    counted(n);
}

fn open_bytes(tag: &str, bytes: &[u8]) -> (PathBuf, Result<Zip, ZipError>) {
    let dir = scratch(tag);
    let p = dir.join("in.zip");
    fs::write(&p, bytes).unwrap();
    let z = Zip::open(&p);
    (dir, z)
}

#[test]
fn zip_entry_count_ffff_with_tiny_directory() {
    let mut z = zip_of(&[f("a", b"a", Comp::Store)], false);
    let e = z.len() - 22;
    put_le(&mut z, e + 8, 0xFFFF, 2);
    put_le(&mut z, e + 10, 0xFFFF, 2);
    reset_peak();
    let (dir, r) = open_bytes("count", &z);
    assert!(matches!(r, Err(ZipError::Damaged(_))), "{:?}", r.err());
    assert!(peak() < 16 << 20, "asked for {} bytes", peak());
    let _ = fs::remove_dir_all(dir);
}

#[test]
fn zip64_entry_count_reserves_memory_first() {
    let mut z = zip_of(&[f("a", b"a", Comp::Store)], true);
    let rec = z.len() - 22 - 20 - 56;
    put_le(&mut z, rec + 24, 2_000_000, 8);
    put_le(&mut z, rec + 32, 2_000_000, 8);
    reset_peak();
    let (dir, r) = open_bytes("count64", &z);
    let big = peak();
    let _ = fs::remove_dir_all(dir);
    assert!(r.is_err());
    assert!(big < 16 << 20, "asked for {big} bytes for a {}-byte file", z.len());
}

#[test]
fn zip64_lying_sizes_are_refused() {
    let text = b"Skyrim Skyrim Skyrim Skyrim Skyrim Skyrim".repeat(50);
    // Uncompressed size smaller than what the data inflates to, larger, and zero.
    for size in [10u64, text.len() as u64 + 1, 0] {
        let (body, mut recs) = locals(&[f("a.txt", &text, Comp::Fixed)]);
        recs[0].size = size;
        let (dir, z) = open_bytes("z64lie", &assemble(body, &recs, true));
        let mut z = z.unwrap();
        let e = z.entries[0].clone();
        assert!(z.read(&e, &Limits::default()).is_err(), "read with size {size} accepted");
        let parent = dir.join("g");
        let r = z.extract_all(&parent.join("out"), &Limits::default());
        assert!(r.is_err(), "extract with size {size} accepted");
        check_extract(&parent, &parent.join("out"), &r).unwrap();
        let _ = fs::remove_dir_all(dir);
    }
    // Compressed size smaller than the real stream.
    let (body, mut recs) = locals(&[f("a.txt", &text, Comp::Fixed)]);
    recs[0].csize = 3;
    let (dir, z) = open_bytes("z64lie3", &assemble(body, &recs, true));
    let parent = dir.join("g");
    let r = z.unwrap().extract_all(&parent.join("out"), &Limits::default());
    assert!(r.is_err());
    check_extract(&parent, &parent.join("out"), &r).unwrap();
    let _ = fs::remove_dir_all(dir);
}

#[test]
fn zip_huge_compressed_size_crashes() {
    // A 150-byte archive whose ZIP64 field says the entry holds 2^50 compressed bytes. Zip::read used to do
    // vec![0; compressed] before reading, asking the allocator for 1 PiB so the process aborted. Refusing it in
    // open() (the data runs past the end of the file) or in read() are both fine.
    let (body, mut recs) = locals(&[f("a.txt", b"abc", Comp::Store)]);
    recs[0].csize = 1 << 50;
    let bytes = assemble(body, &recs, true);
    let (dir, z) = open_bytes("hugecsize", &bytes);
    reset_peak();
    let r = z.and_then(|mut z| {
        let e = z.entries[0].clone();
        z.read(&e, &Limits::default())
    });
    let big = peak();
    let _ = fs::remove_dir_all(dir);
    assert!(r.is_err());
    assert!(big < 16 << 20, "asked for {big} bytes for a {}-byte file", bytes.len());
}

#[test]
fn zip_4gib_compressed_size_allocates_4gib() {
    let (body, mut recs) = locals(&[f("a.txt", b"abc", Comp::Store)]);
    recs[0].csize = 0xFFFF_FFFE;
    let bytes = assemble(body, &recs, false);
    reset_peak();
    let (dir, z) = open_bytes("csize4g", &bytes);
    let r = z.and_then(|mut z| {
        let e = z.entries[0].clone();
        z.read(&e, &Limits::default())
    });
    let big = peak();
    let _ = fs::remove_dir_all(dir);
    assert!(r.is_err());
    assert!(big < 16 << 20, "asked for {big} bytes for a {}-byte file", bytes.len());
}

#[test]
fn zip_stored_size_is_not_checked() {
    // The directory says 1 byte, the stored data is 200 KB (with a matching CRC). The total limit is 1 KB.
    let big = vec![b'x'; 200_000];
    let (body, mut recs) = locals(&[f("a.bin", &big, Comp::Store)]);
    recs[0].size = 1;
    let (dir, z) = open_bytes("storedsize", &assemble(body, &recs, false));
    let parent = dir.join("g");
    let r = z.unwrap().extract_all(&parent.join("out"), &Limits { total: 1024, ratio: 400 });
    let written = fs::metadata(parent.join("out/a.bin")).map(|m| m.len()).unwrap_or(0);
    let _ = fs::remove_dir_all(dir);
    assert!(r.is_err(), "a 1 KB limit let {written} bytes through");
}

#[test]
fn zip_total_size_overflow() {
    let (body, mut recs) = locals(&[f("a", b"a", Comp::Store), f("b", b"b", Comp::Store)]);
    recs[0].size = 1 << 63;
    recs[1].size = 1 << 63;
    let (dir, z) = open_bytes("sumwrap", &assemble(body, &recs, true));
    let parent = dir.join("g");
    let r = catch_unwind(AssertUnwindSafe(|| z.unwrap().extract_all(&parent.join("out"), &Limits::default())));
    let _ = fs::remove_dir_all(dir);
    let r = r.expect("extract_all panicked on the size sum");
    assert!(matches!(&r, Err(ZipError::TooBig(s)) if s == "The archive"), "the total check was skipped: {r:?}");
}

#[test]
fn zip_overlapping_entries_are_refused() {
    let (body, recs) = locals(&[f("a.bin", &vec![0u8; 50_000], Comp::Fixed)]);
    let mut many = Vec::new();
    for k in 0..20 {
        let mut r = recs[0].clone();
        r.name = format!("copy{k}.bin").into_bytes();
        many.push(r);
    }
    let bytes = assemble(body, &many, false);
    let (dir, z) = open_bytes("overlap", &bytes);
    let parent = dir.join("g");
    let r = z.and_then(|mut z| z.extract_all(&parent.join("out"), &Limits::default()));
    let _ = fs::remove_dir_all(dir);
    assert!(r.is_err(), "{}-byte archive with one data block unpacked to 20 files (1 MB)", bytes.len());

    // Overlap on its own: a stored file whose data holds a whole second file (header, name and data), with both
    // listed in the directory. Names match their headers and differ from each other, so only the overlap is wrong.
    let inner_data = b"inner";
    let inner = Rec {
        name: b"b.txt".to_vec(),
        method: 0,
        flags: 0x0800,
        crc: crc32(inner_data),
        csize: inner_data.len() as u64,
        size: inner_data.len() as u64,
        made_by: 3 << 8 | 20,
        ext_attr: 0o100644 << 16,
        offset: 30 + 5,
    };
    let mut outer_data = Vec::new();
    local_header(&mut outer_data, &inner, b"b.txt");
    outer_data.extend_from_slice(inner_data);
    let outer = Rec { name: b"a.txt".to_vec(), crc: crc32(&outer_data), csize: outer_data.len() as u64, size: outer_data.len() as u64, offset: 0, ..inner.clone() };
    let mut body = Vec::new();
    local_header(&mut body, &outer, b"a.txt");
    body.extend_from_slice(&outer_data);
    let (dir, z) = open_bytes("overlap2", &assemble(body, &[outer, inner], false));
    let _ = fs::remove_dir_all(dir);
    assert!(matches!(z, Err(ZipError::Damaged(_))), "a file inside another file's data accepted: {:?}", z.map(|z| z.entries));
}

#[test]
fn zip_local_name_must_match_directory() {
    let mut body = Vec::new();
    let data = b"payload";
    let rec = Rec {
        name: b"Data/plugin.esp".to_vec(),
        method: 0,
        flags: 0x0800,
        crc: crc32(data),
        csize: data.len() as u64,
        size: data.len() as u64,
        made_by: 3 << 8 | 20,
        ext_attr: 0o100644 << 16,
        offset: 0,
    };
    local_header(&mut body, &rec, b"../../../../home/deck/.bashrc");
    body.extend_from_slice(data);
    let (dir, z) = open_bytes("twoface", &assemble(body, &[rec], false));
    let parent = dir.join("g");
    let r = z.and_then(|mut z| z.extract_all(&parent.join("out"), &Limits::default()));
    let _ = fs::remove_dir_all(dir);
    assert!(r.is_err(), "local name and directory name differ, still accepted");
}

#[test]
fn zip_duplicate_names_are_refused() {
    let bytes = zip_of(&[f("Data/a.esp", b"first", Comp::Store), f("Data\\a.esp", b"second", Comp::Store)], false);
    let (dir, z) = open_bytes("dupe", &bytes);
    let parent = dir.join("g");
    let r = z.and_then(|mut z| z.extract_all(&parent.join("out"), &Limits::default()));
    let _ = fs::remove_dir_all(dir);
    assert!(r.is_err(), "duplicate names accepted: {r:?}");
}

#[test]
fn zip_failed_rename_leaves_no_temp_folder() {
    let bytes = zip_of(&[f("a.txt", b"hello", Comp::Store)], false);
    let (dir, z) = open_bytes("renamefail", &bytes);
    let parent = dir.join("g");
    fs::create_dir_all(parent.join("out/already")).unwrap();
    let r = z.unwrap().extract_all(&parent.join("out"), &Limits::default());
    let leftover = parent.join(".out.unpacking").exists();
    let _ = fs::remove_dir_all(dir);
    assert!(r.is_err());
    assert!(!leftover, "temp folder .out.unpacking left behind after {r:?}");
}

// ------------------------------------------------------------------------------------------------------------------
// Big files: unpacking streams, so memory stays flat whatever the file's size.

const BIG: u64 = 300 << 20;

/// Peak resident memory of this process so far, in bytes (VmHWM in /proc/self/status).
fn peak_rss() -> u64 {
    let status = fs::read_to_string("/proc/self/status").unwrap();
    let line = status.lines().find(|l| l.starts_with("VmHWM:")).expect("VmHWM in /proc/self/status");
    line.split_whitespace().nth(1).unwrap().parse::<u64>().unwrap() * 1024
}

/// Builds a valid 300 MiB deflated entry without ever holding 300 MiB: 32 KiB of noise in a stored block, then one
/// fixed-Huffman block of back-references of every length and distance (up to the full 32 KiB window) mixed with
/// short literal runs. Returns the raw DEFLATE stream and the CRC-32 of what it unpacks to.
fn big_stream() -> (Vec<u8>, u32) {
    let mut rng = Rng::new(0xB16_F11E);
    let mut w = BitW::new();
    let mut hist: Vec<u8> = rng.bytes(32 << 10);
    stored_blocks(&mut w, &hist, false);
    let mut crc = Crc32::new();
    crc.update(&hist);
    let mut total = hist.len() as u64;
    w.put(1, 1);
    w.put(1, 2);
    while total < BIG {
        if hist.len() > (4 << 20) {
            hist.drain(..hist.len() - (32 << 10));
        }
        let left = BIG - total;
        let start = hist.len();
        if left < 3 || rng.chance(5) {
            let n = (1 + rng.below(8)).min(left as usize);
            for _ in 0..n {
                let b = rng.next() as u8;
                fixed_lit(&mut w, u32::from(b));
                hist.push(b);
            }
        } else {
            let len = (3 + rng.below(256)).min(left as usize) as u32;
            let dist = *rng.pick(&[1u32, 2, 3, 7, 100, 258, 1000, 4097, 24577, 32768]);
            let li = (0..29).rev().find(|&k| LEN_BASE[k] <= len && (k < 28 || len == 258)).unwrap();
            fixed_lit(&mut w, 257 + li as u32);
            w.put(len - LEN_BASE[li], LEN_EXTRA[li]);
            let di = (0..30).rev().find(|&k| DIST_BASE[k] <= dist).unwrap();
            w.code(di as u32, 5);
            w.put(dist - DIST_BASE[di], DIST_EXTRA[di]);
            let from = hist.len() - dist as usize;
            for k in 0..len as usize {
                let b = hist[from + k];
                hist.push(b);
            }
        }
        crc.update(&hist[start..]);
        total += (hist.len() - start) as u64;
    }
    fixed_lit(&mut w, 256);
    (w.finish(), crc.finish())
}

/// Child half of `zip_big_deflated_entry_streams`: unpacks the archive it is given and writes its peak memory before
/// and after. Does nothing in a normal test run.
#[test]
fn zip_big_entry_child() {
    let Some(input) = std::env::var_os("TURBINE_BIG_ZIP") else { return };
    let dest = PathBuf::from(std::env::var_os("TURBINE_BIG_DEST").unwrap());
    let report = PathBuf::from(std::env::var_os("TURBINE_BIG_REPORT").unwrap());
    let before = peak_rss();
    let written = Zip::open(Path::new(&input)).unwrap().extract_all(&dest, &Limits::default()).unwrap();
    assert_eq!(written, [PathBuf::from("big.bin")]);
    fs::write(report, format!("{before} {}", peak_rss())).unwrap();
}

#[test]
fn zip_big_deflated_entry_streams() {
    let dir = scratch("bigentry");
    let (stream, crc) = big_stream();
    let rec = Rec {
        name: b"big.bin".to_vec(),
        method: 8,
        flags: 0x0800,
        crc,
        csize: stream.len() as u64,
        size: BIG,
        made_by: 3 << 8 | 20,
        ext_attr: 0o100644 << 16,
        offset: 0,
    };
    let mut body = Vec::new();
    local_header(&mut body, &rec, &rec.name);
    body.extend_from_slice(&stream);
    let input = dir.join("big.zip");
    fs::write(&input, assemble(body, &[rec], false)).unwrap();
    drop(stream);

    // Unpack in a fresh process so its peak memory is the unpacking's alone.
    let dest = dir.join("out");
    let report = dir.join("report.txt");
    let out = std::process::Command::new(std::env::current_exe().unwrap())
        .args(["zip_big_entry_child", "--exact", "--test-threads=1"])
        .env("TURBINE_BIG_ZIP", &input)
        .env("TURBINE_BIG_DEST", &dest)
        .env("TURBINE_BIG_REPORT", &report)
        .output()
        .unwrap();
    assert!(out.status.success(), "child failed: {}", String::from_utf8_lossy(&out.stdout));
    let nums: Vec<u64> = fs::read_to_string(&report).unwrap().split(' ').map(|n| n.parse().unwrap()).collect();
    let (before, after) = (nums[0], nums[1]);

    // The unpacked file is complete and correct (checked again here, independently of the CRC check in extract_all).
    let mut f = fs::File::open(dest.join("big.bin")).unwrap();
    let mut buf = vec![0u8; 1 << 20];
    let (mut got, mut len) = (Crc32::new(), 0u64);
    loop {
        let n = std::io::Read::read(&mut f, &mut buf).unwrap();
        if n == 0 {
            break;
        }
        got.update(&buf[..n]);
        len += n as u64;
    }
    let _ = fs::remove_dir_all(&dir);
    assert_eq!(len, BIG);
    assert_eq!(got.finish(), crc);
    eprintln!("zip_big_deflated_entry_streams: peak memory {} KiB before unpacking, {} KiB after", before >> 10, after >> 10);
    assert!(after - before < 8 << 20, "unpacking 300 MiB raised peak memory by {} KiB", (after - before) >> 10);
}

#[test]
fn zip_escaping_names_are_refused() {
    let bad: [&[u8]; 16] = [
        b"/etc/passwd",
        b"C:\\Windows\\x.dll",
        b"c:x",
        b"C:/x",
        b"..\\..\\x",
        b"a\\..\\..\\b",
        b"a/\0b",
        b"\0",
        b"\\\\server\\share\\x",
        b"..",
        b"./../x",
        b"a/b/../../../x",
        b"Data/../../x",
        b"Data\\..\\..\\x",
        b"\\x",
        b"\xff/../\xfe",
    ];
    for name in bad {
        for utf8 in [true, false] {
            let (body, mut recs) = locals(&[f("ok.txt", b"ok", Comp::Store), f("x", b"x", Comp::Store)]);
            recs[1].name = name.to_vec();
            recs[1].flags = if utf8 { 0x0800 } else { 0 };
            let (dir, z) = open_bytes("badname", &assemble(body, &recs, false));
            let _ = fs::remove_dir_all(dir);
            assert!(matches!(z, Err(ZipError::UnsafePath(_))), "{:?} (utf8 flag {utf8}) not refused", String::from_utf8_lossy(name));
        }
    }
}

#[test]
fn zip_odd_but_safe_names_stay_inside() {
    let names: [&[u8]; 12] = [
        b"...",
        b".../x",
        b"a/.../b",
        b"a//b",
        b"./a/./b",
        b" ",
        b"a\\b\\c.txt",
        b"\xe9t\xe9.txt",
        b"\xff\xfe.txt",
        b"dir/",
        b"dir\\",
        b"~/x",
    ];
    for name in names {
        for utf8 in [true, false] {
            let (body, mut recs) = locals(&[f("ok.txt", b"ok", Comp::Store), f("x", b"", Comp::Store)]);
            recs[1].name = name.to_vec();
            recs[1].flags = if utf8 { 0x0800 } else { 0 };
            let body = relocal(&body, &mut recs);
            let bytes = assemble(body, &recs, false);
            let (dir, z) = open_bytes("oddname", &bytes);
            let parent = dir.join("g");
            fs::create_dir_all(&parent).unwrap();
            let dest = parent.join("out");
            // These names are odd but safe, so the archive must open and unpack.
            let r = z.and_then(|mut z| z.extract_all(&dest, &Limits::default()));
            assert!(r.is_ok(), "{:?} (utf8 flag {utf8}) refused: {r:?}", String::from_utf8_lossy(name));
            let check = check_extract(&parent, &dest, &r);
            let _ = fs::remove_dir_all(&dir);
            check.unwrap_or_else(|why| panic!("{:?}: {why}", String::from_utf8_lossy(name)));
        }
    }
}

#[test]
fn zip_dir_and_file_with_same_name_fails_cleanly() {
    let bytes = zip_of(&[f("a", b"file", Comp::Store), f("a/b", b"under", Comp::Store)], false);
    let (dir, z) = open_bytes("dirfile", &bytes);
    let parent = dir.join("g");
    fs::create_dir_all(&parent).unwrap();
    let r = z.unwrap().extract_all(&parent.join("out"), &Limits::default());
    let check = check_extract(&parent, &parent.join("out"), &r);
    let _ = fs::remove_dir_all(dir);
    assert!(r.is_err());
    check.unwrap();
}

#[test]
fn zip_odd_structures_never_panic() {
    let one = zip_of(&[f("a", b"abc", Comp::Fixed)], false);
    let mut cases: Vec<Vec<u8>> = vec![
        vec![],
        b"PK\x05\x06".to_vec(),
        {
            let mut v = vec![0u8; 22];
            v[..4].copy_from_slice(b"PK\x05\x06");
            v
        },
        // End record only, claiming a directory past the end of the file.
        {
            let mut v = vec![0u8; 22];
            v[..4].copy_from_slice(b"PK\x05\x06");
            put_le(&mut v, 10, 1, 2);
            put_le(&mut v, 12, 0xFFFF_FFFF, 4);
            put_le(&mut v, 16, 0xFFFF_FFFF, 4);
            v
        },
        // ZIP64 locator pointing far past the file, and at itself.
        {
            let mut v = zip_of(&[f("a", b"a", Comp::Store)], true);
            let loc = v.len() - 22 - 20;
            put_le(&mut v, loc + 8, u64::MAX, 8);
            v
        },
        {
            let mut v = zip_of(&[f("a", b"a", Comp::Store)], true);
            let loc = v.len() - 22 - 20;
            put_le(&mut v, loc + 8, loc as u64, 8);
            v
        },
        // Directory offset + size that overflow u64.
        {
            let mut v = zip_of(&[f("a", b"a", Comp::Store)], true);
            let rec = v.len() - 22 - 20 - 56;
            put_le(&mut v, rec + 40, u64::MAX, 8);
            put_le(&mut v, rec + 48, 2, 8);
            v
        },
        // Local offset pointing at the end record, and past the end of the file.
        {
            let (body, mut recs) = locals(&[f("a", b"a", Comp::Store)]);
            recs[0].offset = 0xFFFF_FFF0;
            assemble(body, &recs, true)
        },
        {
            let (body, mut recs) = locals(&[f("a", b"a", Comp::Store)]);
            recs[0].offset = u64::MAX;
            assemble(body, &recs, true)
        },
        // Unknown method, encryption flag, the end record found inside a comment.
        {
            let (body, mut recs) = locals(&[f("a", b"a", Comp::Store)]);
            recs[0].method = 99;
            assemble(body, &recs, false)
        },
        {
            let (body, mut recs) = locals(&[f("a", b"a", Comp::Store)]);
            recs[0].flags |= 1;
            assemble(body, &recs, false)
        },
        {
            let mut v = one.clone();
            let n = v.len();
            put_le(&mut v, n - 2, 30, 2);
            v.extend_from_slice(b"PK\x05\x06PK\x05\x06PK\x05\x06PK\x05\x06xx");
            v
        },
    ];
    // Every prefix of a valid archive.
    for k in 0..one.len() {
        cases.push(one[..k].to_vec());
    }
    let dir = scratch("oddstruct");
    for (i, c) in cases.iter().enumerate() {
        let p = dir.join("in.zip");
        fs::write(&p, c).unwrap();
        let opened = case("zip-odd", 0, i, c, Duration::from_secs(2), || Zip::open(&p));
        if let Ok(mut z) = opened {
            assert_sizes_fit(&z, c.len() as u64, i);
            let parent = dir.join("g");
            let _ = fs::remove_dir_all(&parent);
            fs::create_dir_all(&parent).unwrap();
            let r = case("zip-odd-extract", 0, i, c, Duration::from_secs(2), || z.extract_all(&parent.join("out"), &Limits::default()));
            check_extract(&parent, &parent.join("out"), &r).unwrap_or_else(|why| panic!("case {i}: {why}"));
        }
    }
    let _ = fs::remove_dir_all(dir);
}

// ------------------------------------------------------------------------------------------------------------------
// Fuzz: JSON.

const STR_POOL: [char; 24] =
    ['a', 'b', 'Z', '0', '9', ' ', '"', '\\', '/', '\n', '\r', '\t', '\u{1}', '\u{1f}', '\u{7f}', 'é', 'ß', '€', '\u{2028}', '\u{feff}', '😀', '{', '}', ':'];

fn rand_string(rng: &mut Rng, max: usize) -> String {
    (0..rng.below(max + 1)).map(|_| *rng.pick(&STR_POOL)).collect()
}

fn gen_json(rng: &mut Rng, depth: usize) -> Json {
    match rng.below(if depth >= 5 { 4 } else { 6 }) {
        0 => Json::Null,
        1 => Json::Bool(rng.chance(50)),
        2 => Json::Num(match rng.below(4) {
            0 => (rng.next() as i32) as f64,
            1 => (rng.next() as i16) as f64 / 8.0,
            2 => (rng.next() % 1000) as f64 * 1e15,
            _ => f64::from_bits(rng.next() >> 2 | 0x3000_0000_0000_0000),
        }),
        3 => Json::Str(rand_string(rng, 12)),
        4 => Json::Arr((0..rng.below(5)).map(|_| gen_json(rng, depth + 1)).collect()),
        _ => Json::Obj((0..rng.below(5)).map(|_| (rand_string(rng, 6), gen_json(rng, depth + 1))).collect()),
    }
}

fn ws(rng: &mut Rng, out: &mut String) {
    if rng.chance(30) {
        out.push_str(rng.pick(&[" ", "\t", "\r\n", "\n  ", ""]));
    }
}

fn json_str(rng: &mut Rng, s: &str, out: &mut String) {
    out.push('"');
    for c in s.chars() {
        let escape = matches!(c, '"' | '\\') || (c as u32) < 0x20 || rng.chance(10);
        if !escape {
            out.push(c);
            continue;
        }
        match c {
            '"' => out.push_str("\\\""),
            '\\' => out.push_str("\\\\"),
            '/' if rng.chance(50) => out.push_str("\\/"),
            '\n' if rng.chance(50) => out.push_str("\\n"),
            '\t' if rng.chance(50) => out.push_str("\\t"),
            _ => {
                let mut buf = [0u16; 2];
                for u in c.encode_utf16(&mut buf) {
                    if rng.chance(50) {
                        out.push_str(&format!("\\u{u:04x}"));
                    } else {
                        out.push_str(&format!("\\u{u:04X}"));
                    }
                }
            }
        }
    }
    out.push('"');
}

fn ser_json(rng: &mut Rng, v: &Json, out: &mut String) {
    ws(rng, out);
    match v {
        Json::Null => out.push_str("null"),
        Json::Bool(b) => out.push_str(if *b { "true" } else { "false" }),
        Json::Num(n) => out.push_str(&format!("{n:?}")),
        Json::Str(s) => json_str(rng, s, out),
        Json::Arr(items) => {
            out.push('[');
            for (k, it) in items.iter().enumerate() {
                if k > 0 {
                    out.push(',');
                }
                ser_json(rng, it, out);
            }
            ws(rng, out);
            out.push(']');
        }
        Json::Obj(items) => {
            out.push('{');
            for (k, (key, it)) in items.iter().enumerate() {
                if k > 0 {
                    out.push(',');
                }
                ws(rng, out);
                json_str(rng, key, out);
                ws(rng, out);
                out.push(':');
                ser_json(rng, it, out);
            }
            ws(rng, out);
            out.push('}');
        }
    }
    ws(rng, out);
}

#[test]
fn fuzz_json() {
    let n = guarded("fuzz_json", Duration::from_secs(40), || {
        let seed = 0x750A_0001;
        let mut rng = Rng::new(seed);
        let slow = Duration::from_secs(1);
        let mut runs = 0u64;
        let mut texts = Vec::new();
        for i in 0..6_000 {
            let v = gen_json(&mut rng, 0);
            let mut t = String::new();
            if rng.chance(10) {
                t.push('\u{feff}');
            }
            ser_json(&mut rng, &v, &mut t);
            let got = case("json-valid", seed, i, t.as_bytes(), slow, || json::parse(&t));
            assert_eq!(got.as_ref().ok(), Some(&v), "round trip failed, iteration {i}: {t:?} gave {got:?}");
            // And the engine's own writer, for strings.
            if let Json::Str(s) = &v {
                assert_eq!(json::parse(&json::quote(s)).ok(), Some(Json::Str(s.clone())));
            }
            texts.push(t.into_bytes());
            runs += 1;
        }
        let seed = 0x750A_0002;
        let mut rng = Rng::new(seed);
        for i in 0..40_000 {
            let pick = rng.below(texts.len());
            let m = mutate(&mut rng, &texts[pick], false);
            let t = String::from_utf8_lossy(&m);
            let _ = case("json-mutated", seed, i, &m, slow, || json::parse(&t));
            runs += 1;
        }
        runs
    });
    counted(n);
    eprintln!("fuzz_json: {n} runs clean");
}

#[test]
fn json_hostile_shapes() {
    let ten_mb = 10 << 20;
    // (text, should it parse)
    let cases: Vec<(String, bool)> = vec![
        ("[".repeat(ten_mb), false),
        ("{\"a\":".repeat(ten_mb / 5), false),
        (format!("{}{}", "[".repeat(128), "]".repeat(128)), true),
        (format!("{}{}", "[".repeat(130), "]".repeat(130)), false),
        (format!("[{}]", "[],".repeat(ten_mb / 3) + "[]"), true),
        (format!("\"{}\"", "\\u0041".repeat(ten_mb / 6)), true),
        (format!("\"{}", "a".repeat(ten_mb)), false),
        (" ".repeat(ten_mb), false),
        (format!("{}1", "-".repeat(ten_mb)), false),
        ("1".repeat(ten_mb), true),
        ("\u{feff}{}".into(), true),
        ("\u{feff}\u{feff}{}".into(), false),
        ("\r\n{\r\n\"a\"\r\n:\r\n1\r\n}\r\n".into(), true),
        ("\"\\ud83d\"".into(), false),
        ("\"\\udc00\"".into(), false),
        ("\"\\ud83d\\u0041\"".into(), false),
        ("\"\\u12\"".into(), false),
        ("\"\\".into(), false),
        ("tru".into(), false),
        ("nulll".into(), false),
        ("-".into(), false),
        ("{\"a\":1,}".into(), false),
        ("{,}".into(), false),
        ("\"\u{0}\"".into(), false),
        ("\"\u{7f}\"".into(), true),
    ];
    for (i, (t, ok)) in cases.iter().enumerate() {
        let r = case("json-hostile", 0, i, &t.as_bytes()[..t.len().min(64)], Duration::from_secs(3), || json::parse(t));
        assert_eq!(r.is_ok(), *ok, "case {i} ({:?}...): {:?}", &t[..t.len().min(30)], r.err());
    }
}

#[test]
fn json_rejects_invalid_numbers_and_escapes() {
    for bad in ["01", "1.", "1.e5", "-01", "[1.]", "\"\\u+041\"", "\"\\u+04a\""] {
        assert!(json::parse(bad).is_err(), "{bad:?} accepted as {:?}", json::parse(bad));
    }
}

// ------------------------------------------------------------------------------------------------------------------
// Fuzz: VDF.

const VDF_POOL: [char; 18] = ['a', 'Z', '0', '_', '.', ' ', '"', '\\', '\t', '\n', 'é', '😀', '{', '}', '/', '[', '$', '\u{feff}'];

fn gen_vdf(rng: &mut Rng, depth: usize) -> Vec<(String, Vdf)> {
    (0..rng.below(5))
        .map(|_| {
            let key = (0..rng.below(8)).map(|_| *rng.pick(&VDF_POOL)).collect();
            let v = if depth < 5 && rng.chance(35) {
                Vdf::Obj(gen_vdf(rng, depth + 1))
            } else {
                Vdf::Str((0..rng.below(10)).map(|_| *rng.pick(&VDF_POOL)).collect())
            };
            (key, v)
        })
        .collect()
}

fn vdf_sep(rng: &mut Rng, out: &mut String) {
    out.push_str(rng.pick(&[" ", "\t", "\r\n", "\n", " // note { \"x\n", "\n\t\t", "\t\t"]));
}

fn vdf_str(rng: &mut Rng, s: &str, out: &mut String) {
    let bare = !s.is_empty() && s.chars().all(|c| c.is_ascii_alphanumeric() || c == '_' || c == '.') && !s.starts_with("//");
    if bare && rng.chance(40) {
        out.push_str(s);
        return;
    }
    out.push('"');
    for c in s.chars() {
        match c {
            '"' => out.push_str("\\\""),
            '\\' => out.push_str("\\\\"),
            '\n' if rng.chance(50) => out.push_str("\\n"),
            '\t' if rng.chance(50) => out.push_str("\\t"),
            c => out.push(c),
        }
    }
    out.push('"');
}

fn ser_vdf(rng: &mut Rng, items: &[(String, Vdf)], out: &mut String) {
    for (k, v) in items {
        vdf_sep(rng, out);
        vdf_str(rng, k, out);
        vdf_sep(rng, out);
        match v {
            Vdf::Str(s) => vdf_str(rng, s, out),
            Vdf::Obj(inner) => {
                out.push('{');
                ser_vdf(rng, inner, out);
                vdf_sep(rng, out);
                out.push('}');
            }
        }
    }
}

#[test]
fn fuzz_vdf() {
    let n = guarded("fuzz_vdf", Duration::from_secs(40), || {
        let seed = 0x0DF0_0001;
        let mut rng = Rng::new(seed);
        let slow = Duration::from_secs(1);
        let mut runs = 0u64;
        let mut texts = Vec::new();
        for i in 0..6_000 {
            let items = gen_vdf(&mut rng, 0);
            let mut t = String::new();
            if rng.chance(10) {
                t.push('\u{feff}');
            }
            ser_vdf(&mut rng, &items, &mut t);
            let got = case("vdf-valid", seed, i, t.as_bytes(), slow, || vdf::parse(&t));
            assert_eq!(got.as_ref().ok(), Some(&Vdf::Obj(items)), "round trip failed, iteration {i}: {t:?}");
            texts.push(t.into_bytes());
            runs += 1;
        }
        let seed = 0x0DF0_0002;
        let mut rng = Rng::new(seed);
        for i in 0..40_000 {
            let pick = rng.below(texts.len());
            let m = mutate(&mut rng, &texts[pick], false);
            let t = String::from_utf8_lossy(&m);
            let _ = case("vdf-mutated", seed, i, &m, slow, || vdf::parse(&t));
            runs += 1;
        }
        runs
    });
    counted(n);
    eprintln!("fuzz_vdf: {n} runs clean");
}

#[test]
fn vdf_hostile_shapes() {
    let ten_mb = 10 << 20;
    let bad: Vec<String> = vec![
        "\"a\" \"b".into(),
        "\"a\" { \"b\" \"c".into(),
        "\"a\" \"b\\".into(),
        "\"a\"".into(),
        "\"a\" {".into(),
        "{".into(),
        "}".into(),
        "\"a\" }".into(),
        format!("\"{}", "x".repeat(ten_mb)),
        "\"a\" { \"b\" }".into(),
    ];
    for (i, t) in bad.iter().enumerate() {
        let r = case("vdf-bad", 0, i, &t.as_bytes()[..t.len().min(64)], Duration::from_secs(3), || vdf::parse(t));
        assert!(r.is_err(), "case {i} accepted");
    }
    let good: Vec<&str> = vec![
        "\u{feff}\"a\" \"b\"",
        "\"a\"\r\n{\r\n\t\"b\"\t\t\"c\"\r\n}\r\n",
        "\"a\" \"b\" [$WIN32]",
        "\"a\" \"b\" [$WIN32",
        "// only a comment",
        "",
        "a b c d",
        "\"a\" \"\u{fffd}\u{fffd}\"",
    ];
    for t in good {
        assert!(vdf::parse(t).is_ok(), "{t:?} refused");
    }
    // Nesting works up to the limit of 128 levels (real Steam files use a handful), and one more is refused.
    let deep = |n: usize| format!("{}\"k\" \"v\"{}", "\"a\" {".repeat(n), "}".repeat(n));
    assert!(vdf::parse(&deep(128)).is_ok());
    assert!(vdf::parse(&deep(129)).is_err());
    // Long flat files stay fast.
    let wide = "\"k\" \"v\"\n".repeat(500_000);
    let t = Instant::now();
    assert_eq!(vdf::parse(&wide).unwrap().entries().len(), 500_000);
    assert!(t.elapsed() < Duration::from_secs(3));
}

#[test]
fn vdf_deep_nesting_is_refused() {
    // Run on a normal 2 MiB thread, like the app's worker threads. This aborts the process with a stack overflow.
    let t = format!("{}{}", "\"a\" {".repeat(1_000_000), "}".repeat(1_000_000));
    let r = std::thread::spawn(move || vdf::parse(&t).is_err()).join().unwrap();
    assert!(r, "a million levels of nesting accepted");
}

// ------------------------------------------------------------------------------------------------------------------
// Fuzz: the Steam and Heroic scans, with damaged launcher files on disk.

struct Home(PathBuf);

impl Drop for Home {
    fn drop(&mut self) {
        let _ = fs::remove_dir_all(&self.0);
    }
}

fn put(path: &Path, bytes: &[u8]) {
    fs::create_dir_all(path.parent().unwrap()).unwrap();
    fs::write(path, bytes).unwrap();
}

#[test]
fn fuzz_scan_files() {
    let n = guarded("fuzz_scan_files", Duration::from_secs(40), || {
        let home = Home(scratch("scanhome"));
        let h = &home.0;
        let steam = h.join(".local/share/Steam");
        let sapps = steam.join("steamapps");
        let heroic = h.join(".config/heroic");
        let game = h.join("Games/Some Game");
        put(&sapps.join("common/Game/game.exe"), b"");
        put(&game.join("game.exe"), b"");
        let lib = format!("\"libraryfolders\"\n{{\n\"0\"\n{{\n\"path\" \"{}\"\n\"apps\" {{ \"10\" \"1\" }}\n}}\n\"1\" \"{}\"\n}}\n", steam.display(), steam.display());
        let acf = "\"AppState\"\n{\n\t\"appid\"\t\"10\"\n\t\"name\"\t\"Game\"\n\t\"StateFlags\"\t\"4\"\n\t\"installdir\"\t\"Game\"\n\t\"buildid\"\t\"1\"\n}\n";
        let g = game.display().to_string();
        let epic = format!(r#"{{"G": {{"title": "Game", "install_path": "{g}", "platform": "Windows", "version": "1", "is_dlc": false}}}}"#);
        let gog = format!(r#"{{"installed": [{{"appName": "1", "install_path": "{g}", "platform": "linux", "version": "1"}}]}}"#);
        let side = format!(r#"{{"games": [{{"app_name": "s", "title": "S", "folder_name": "{g}", "is_installed": true, "install": {{"platform": "Windows"}}}}]}}"#);
        let lib_t = r#"{"games": [{"app_name": "1", "title": "Gog Game"}]}"#;
        let files: Vec<(PathBuf, Vec<u8>)> = vec![
            (sapps.join("libraryfolders.vdf"), lib.into_bytes()),
            (sapps.join("appmanifest_10.acf"), acf.as_bytes().to_vec()),
            (heroic.join("legendaryConfig/legendary/installed.json"), epic.into_bytes()),
            (heroic.join("gog_store/installed.json"), gog.into_bytes()),
            (heroic.join("store_cache/gog_library.json"), lib_t.as_bytes().to_vec()),
            (heroic.join("sideload_apps/library.json"), side.into_bytes()),
        ];
        for (p, b) in &files {
            put(p, b);
        }
        let clean = turbine_engine::scan_games(h);
        assert_eq!(clean.games.len(), 4, "{:?} {:?}", clean.games, clean.problems);
        let seed = 0x5CA0_0001;
        let mut rng = Rng::new(seed);
        let (mut runs, mut outside) = (0u64, 0u64);
        for i in 0..1_500 {
            for (p, b) in &files {
                let m = if rng.chance(50) { mutate(&mut rng, b, false) } else { b.clone() };
                put(p, &m);
            }
            let scan = case("scan", seed, i, b"", Duration::from_secs(3), || turbine_engine::scan_games(h));
            for game in &scan.games {
                if game.store == turbine_engine::Store::Steam {
                    let common = game.library.join("steamapps/common");
                    let inside = game.install_dir.starts_with(&common)
                        && game.install_dir.strip_prefix(&common).unwrap().components().all(|c| matches!(c, Component::Normal(_)));
                    if !inside {
                        outside += 1; // known defect D7
                    }
                }
                let _ = turbine_engine::json::parse(&game.to_json()).expect("game JSON must stay valid");
            }
            let _ = turbine_engine::json::parse(&scan.games_json()).expect("games JSON must stay valid");
            runs += 1;
        }
        eprintln!("fuzz_scan_files: {runs} scans clean, {outside} Steam games outside their library (D7)");
        runs
    });
    counted(n);
}

#[test]
fn scan_survives_invalid_utf8_bom_and_crlf() {
    let home = Home(scratch("scanodd"));
    let h = &home.0;
    let sapps = h.join(".local/share/Steam/steamapps");
    put(&sapps.join("common/Game/game.exe"), b"");
    put(&sapps.join("libraryfolders.vdf"), b"\xef\xbb\xbf\"libraryfolders\"\r\n{\r\n\"0\" { \"path\" \"\xff\xfe/nowhere\" }\r\n}\r\n");
    put(&sapps.join("appmanifest_10.acf"), b"\xef\xbb\xbf\"AppState\"\r\n{\r\n\"appid\" \"10\"\r\n\"name\" \"G\xffame\"\r\n\"installdir\" \"Game\"\r\n}\r\n");
    put(&sapps.join("appmanifest_11.acf"), b"\"AppState\" { \"appid\" \"11\" \"name\" \"unterminated");
    put(&sapps.join("appmanifest_12.acf"), &[0xff; 4096]);
    put(&sapps.join("appmanifest_13.acf"), b"");
    let heroic = h.join(".config/heroic");
    put(&heroic.join("legendaryConfig/legendary/installed.json"), b"\xef\xbb\xbf{\r\n\"G\": {\"install_path\": \"\xff\"}}");
    put(&heroic.join("gog_store/installed.json"), b"{\"installed\": 5}");
    put(&heroic.join("sideload_apps/library.json"), b"[[[[[[[[");
    let scan = turbine_engine::scan_games(h);
    assert_eq!(scan.games.len(), 1, "{:?}", scan.games);
    assert_eq!(scan.games[0].name, "G\u{fffd}ame");
    assert!(scan.problems.iter().any(|p| p.contains("appmanifest_11.acf")), "{:?}", scan.problems);
}

#[test]
#[ignore = "DEFECT: Steam's installdir is joined without checks, so '/abs' or '../..' puts a game's folder outside its library"]
fn steam_installdir_cannot_leave_the_library() {
    let home = Home(scratch("installdir"));
    let h = &home.0;
    let sapps = h.join(".local/share/Steam/steamapps");
    let victim = h.join("Documents");
    put(&victim.join("taxes.txt"), b"x");
    put(&sapps.join("common/Real/game.exe"), b"");
    put(&sapps.join("appmanifest_10.acf"), format!("\"AppState\" {{ \"appid\" \"10\" \"name\" \"Abs\" \"installdir\" \"{}\" }}", victim.display()).as_bytes());
    put(&sapps.join("appmanifest_11.acf"), b"\"AppState\" { \"appid\" \"11\" \"name\" \"Dots\" \"installdir\" \"../../../../Documents\" }");
    put(&sapps.join("appmanifest_12.acf"), b"\"AppState\" { \"appid\" \"../../../../Documents\" \"name\" \"Id\" \"installdir\" \"Real\" }");
    let scan = turbine_engine::scan_games(h);
    for g in &scan.games {
        let common = g.library.join("steamapps/common");
        let rel = g.install_dir.strip_prefix(&common).ok();
        let inside = rel.is_some_and(|r| r.components().all(|c| matches!(c, Component::Normal(_))));
        assert!(inside, "{} has its folder at {}", g.name, g.install_dir.display());
        assert!(g.store_id.chars().all(|c| c.is_ascii_digit()), "app ID {:?} accepted", g.store_id);
    }
}
