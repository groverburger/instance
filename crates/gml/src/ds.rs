//! Data structures (`ds_*`, chapter 411): stacks, queues, lists, maps, priority
//! queues and grids.
//!
//! Each kind has its own id space starting at 0; ids of destroyed structures are
//! reused (lowest free id first).

use crate::value::Value;
use std::cmp::Ordering;
use std::collections::VecDeque;

/// Slot storage with id reuse.
#[derive(Clone, Debug)]
pub struct Slab<T> {
    items: Vec<Option<T>>,
}

impl<T> Default for Slab<T> {
    fn default() -> Self {
        Slab { items: Vec::new() }
    }
}

impl<T> Slab<T> {
    /// Inserts a structure, returning its id.
    pub fn insert(&mut self, v: T) -> usize {
        if let Some(i) = self.items.iter().position(|x| x.is_none()) {
            self.items[i] = Some(v);
            i
        } else {
            self.items.push(Some(v));
            self.items.len() - 1
        }
    }
    /// Gets a structure.
    pub fn get(&self, id: usize) -> Option<&T> {
        self.items.get(id)?.as_ref()
    }
    /// Gets a structure mutably.
    pub fn get_mut(&mut self, id: usize) -> Option<&mut T> {
        self.items.get_mut(id)?.as_mut()
    }
    /// Removes a structure.
    pub fn remove(&mut self, id: usize) -> Option<T> {
        self.items.get_mut(id)?.take()
    }
    /// Iterates over live structures.
    pub fn iter(&self) -> impl Iterator<Item = (usize, &T)> {
        self.items
            .iter()
            .enumerate()
            .filter_map(|(i, x)| x.as_ref().map(|x| (i, x)))
    }
    /// Removes everything.
    pub fn clear(&mut self) {
        self.items.clear()
    }
}

/// A grid of values (column `x`, row `y`).
#[derive(Clone, Debug, Default)]
pub struct Grid {
    /// Width.
    pub w: usize,
    /// Height.
    pub h: usize,
    /// Cells, row-major (`x + y * w`).
    pub cells: Vec<Value>,
}

impl Grid {
    /// A `w`×`h` grid of zeros.
    pub fn new(w: usize, h: usize) -> Self {
        Grid {
            w,
            h,
            cells: vec![Value::Real(0.0); w * h],
        }
    }
    /// Resizes keeping cell values at their coordinates; new cells are 0.
    pub fn resize(&mut self, w: usize, h: usize) {
        let mut cells = vec![Value::Real(0.0); w * h];
        for y in 0..h.min(self.h) {
            for x in 0..w.min(self.w) {
                cells[x + y * w] = std::mem::take(&mut self.cells[x + y * self.w]);
            }
        }
        *self = Grid { w, h, cells };
    }
    /// Cell reference if in range.
    pub fn get(&self, x: i64, y: i64) -> Option<&Value> {
        if x < 0 || y < 0 || x as usize >= self.w || y as usize >= self.h {
            return None;
        }
        self.cells.get(x as usize + y as usize * self.w)
    }
    /// Mutable cell reference if in range.
    pub fn get_mut(&mut self, x: i64, y: i64) -> Option<&mut Value> {
        if x < 0 || y < 0 || x as usize >= self.w || y as usize >= self.h {
            return None;
        }
        self.cells.get_mut(x as usize + y as usize * self.w)
    }
}

/// All data structures of a game.
#[derive(Clone, Debug)]
pub struct DsStore {
    /// Stacks (top = last element).
    pub stacks: Slab<Vec<Value>>,
    /// Queues (head = front).
    pub queues: Slab<VecDeque<Value>>,
    /// Lists.
    pub lists: Slab<Vec<Value>>,
    /// Maps: key/value pairs sorted by key (duplicates allowed).
    pub maps: Slab<Vec<(Value, Value)>>,
    /// Priority queues: (value, priority) in insertion order.
    pub priorities: Slab<Vec<(Value, Value)>>,
    /// Grids.
    pub grids: Slab<Grid>,
    /// Precision for comparing reals (`ds_set_precision`).
    pub precision: f64,
}

impl Default for DsStore {
    fn default() -> Self {
        DsStore {
            stacks: Slab::default(),
            queues: Slab::default(),
            lists: Slab::default(),
            maps: Slab::default(),
            priorities: Slab::default(),
            grids: Slab::default(),
            precision: 1e-7,
        }
    }
}

impl DsStore {
    /// Equality used by data structures: reals within `precision`, strings exactly,
    /// mixed types never.
    pub fn eq(&self, a: &Value, b: &Value) -> bool {
        match (a, b) {
            (Value::Real(x), Value::Real(y)) => (x - y).abs() <= self.precision || x == y,
            (Value::Str(x), Value::Str(y)) => x == y,
            _ => false,
        }
    }

    /// Ordering consistent with [`eq`](Self::eq) (values equal within precision
    /// compare `Equal`).
    pub fn cmp(&self, a: &Value, b: &Value) -> Ordering {
        if self.eq(a, b) {
            Ordering::Equal
        } else {
            a.total_cmp(b)
        }
    }

    /// Index of the first map entry whose key equals `key`.
    pub fn map_find(&self, entries: &[(Value, Value)], key: &Value) -> Option<usize> {
        let lo = entries.partition_point(|(k, _)| self.cmp(k, key) == Ordering::Less);
        (lo < entries.len() && self.eq(&entries[lo].0, key)).then_some(lo)
    }

    /// Removes everything (e.g. on game restart).
    pub fn clear(&mut self) {
        *self = DsStore {
            precision: self.precision,
            ..DsStore::default()
        };
    }
}

// ---------------------------------------------------------------- serialisation
//
// Format (implementation-defined): a string of upper-case hex digits. Each value is
// `0` + 16 hex digits (IEEE bits) for a real, or `1` + 8 hex digits length + 2 hex
// digits per byte for a string. Counts are 8 hex digits. Each structure starts with a
// 4-digit kind tag.

pub(crate) const TAG_STACK: u16 = 0x0101;
pub(crate) const TAG_QUEUE: u16 = 0x0102;
pub(crate) const TAG_LIST: u16 = 0x0103;
pub(crate) const TAG_MAP: u16 = 0x0104;
pub(crate) const TAG_PRIORITY: u16 = 0x0105;
pub(crate) const TAG_GRID: u16 = 0x0106;

/// Writer for the serialisation format.
#[derive(Default)]
pub(crate) struct Writer(pub String);

impl Writer {
    pub fn tag(&mut self, t: u16) {
        self.0.push_str(&format!("{:04X}", t));
    }
    pub fn count(&mut self, n: usize) {
        self.0.push_str(&format!("{:08X}", n));
    }
    pub fn value(&mut self, v: &Value) {
        match v {
            Value::Real(x) => self.0.push_str(&format!("0{:016X}", x.to_bits())),
            Value::Str(s) => {
                self.0.push('1');
                self.count(s.len());
                for b in s.iter() {
                    self.0.push_str(&format!("{:02X}", b));
                }
            }
        }
    }
}

/// Reader for the serialisation format.
pub(crate) struct Reader<'a> {
    s: &'a [u8],
    pos: usize,
}

impl<'a> Reader<'a> {
    pub fn new(s: &'a [u8]) -> Self {
        Reader { s, pos: 0 }
    }
    fn hex(&mut self, n: usize) -> Option<u64> {
        let t = self.s.get(self.pos..self.pos + n)?;
        self.pos += n;
        u64::from_str_radix(std::str::from_utf8(t).ok()?, 16).ok()
    }
    pub fn tag(&mut self, expect: u16) -> Option<()> {
        (self.hex(4)? == expect as u64).then_some(())
    }
    pub fn count(&mut self) -> Option<usize> {
        let n = self.hex(8)? as usize;
        // Sanity bound: every element needs at least 17 characters.
        (n <= self.s.len()).then_some(n)
    }
    pub fn value(&mut self) -> Option<Value> {
        match self.hex(1)? {
            0 => Some(Value::Real(f64::from_bits(self.hex(16)?))),
            1 => {
                let n = self.count()?;
                let mut v = Vec::with_capacity(n);
                for _ in 0..n {
                    v.push(self.hex(2)? as u8);
                }
                Some(Value::from(v))
            }
            _ => None,
        }
    }
    pub fn done(&self) -> bool {
        self.pos == self.s.len()
    }
}
