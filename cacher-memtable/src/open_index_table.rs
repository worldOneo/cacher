pub struct OpenIndexTable {
  data: Vec<u64>,
  data_cap: u64,
  data_mask: u64,
  cap: u64,
  cap_mask: u64,
  size: u64,
  free_value: u64,
  free_set: bool,
}

fn scramble(k: u64) -> u64 {
  let hash = k * 0x9E3779B9;
  hash * (hash >> 16)
}
const FREE_KEY: u64 = 0;
impl OpenIndexTable {
  pub fn new() -> OpenIndexTable {
    let initial_cap: u64 = 64;
    OpenIndexTable {
      data: std::vec::from_elem(0, initial_cap as usize),
      data_mask: initial_cap - 1,
      data_cap: initial_cap,
      cap: ((initial_cap >> 1) / 16) * 14, // 87.5% fill
      cap_mask: (initial_cap >> 1) - 1,
      free_value: 0,
      free_set: false,
      size: 0,
    }
  }

  fn index(&self, k: u64) -> u64 {
    (scramble(k) & self.cap_mask) << 1
  }

  fn next(&self, index: u64) -> u64 {
    (index + 2) & self.data_mask
  }

  pub fn get(&self, key: u64) -> (u64, bool) {
    if key == FREE_KEY {
      return (self.free_value, self.free_set);
    }
    let mut index = self.index(key);
    loop {
      let data = &self.data;
      let assigned_key = data[index as usize];
      if assigned_key == FREE_KEY {
        return (0, false);
      }
      if assigned_key == key {
        return (data[index as usize + 1], true);
      }
      index = self.next(index);
    }
  }

  pub fn insert(&mut self, new_key: u64, v: u64) {
    if new_key == FREE_KEY {
      self.free_value = v;
      self.free_set = true;
      return;
    }
    let mut index = self.index(new_key);
    loop {
      let assigned_key = self.data[index as usize];
      if assigned_key == new_key || assigned_key == FREE_KEY {
        if assigned_key == FREE_KEY {
          self.size += 1;
          self.data[index as usize] = new_key;
        }
        self.data[index as usize + 1] = v;
        break;
      }
      index = self.next(index);
    }
    self.expand();
  }

  pub fn delete(&mut self, key: u64) -> (u64, bool) {
    if key == FREE_KEY {
      self.free_set = false;
      return (self.free_value, true);
    }
    let mut index = self.index(key);
    let v;
    let found;
    loop {
      let assigned_key = self.data[index as usize];
      if assigned_key == key || assigned_key == FREE_KEY {
        if assigned_key == FREE_KEY {
          return (0, false);
        }
        found = true;
        self.data[index as usize] = 0;
        v = self.data[index as usize + 1];
        break;
      }
      index = self.next(index);
    }
    self.unshift(index);
    return (v, found);
  }

  fn unshift(&mut self, current: u64) {
    let mut current = current;
    let mut key;
    loop {
      let last = current;
      current = self.next(current);
      loop {
        key = self.data[current as usize];
        if key == FREE_KEY {
          self.data[key as usize] = FREE_KEY;
          return;
        }
        let slot = self.index(key);
        if last < current {
          if last >= slot || slot > current {
            break;
          }
        } else if last >= slot && slot > current {
          break;
        }
        current = self.next(current);
      }
      self.data[last as usize] = key;
      self.data[last as usize + 1] = self.data[current as usize + 1];
    }
  }

  fn expand(&mut self) {
    if self.size <= self.cap {
      return;
    }

    let data_cap = self.data_cap * 2;
    let cap = self.cap * 2;
    let mut new = OpenIndexTable {
      data: std::vec::from_elem(0, data_cap as usize),
      data_cap: data_cap,
      data_mask: data_cap - 1,
      cap_mask: (data_cap >> 1) - 1,
      cap: cap,
      size: self.size,
      free_value: self.free_value,
      free_set: self.free_set,
    };
    let mut n = 0;
    while n < self.data_cap {
      new.insert(self.data[n as usize], self.data[n as usize + 1]);
      n += 2;
    }
    *self = new;
  }
}

extern crate test;
use std::collections::HashMap;
use test::Bencher;

#[test]
fn test_table_insert() {
  let mut table = OpenIndexTable::new();
  table.insert(1, 2);
  table.insert(2, 3);
  table.insert(3, 4);
  table.insert(4, 5);
  assert_eq!(table.get(1), (2, true));
  assert_eq!(table.get(2), (3, true));
  assert_eq!(table.get(3), (4, true));
  assert_eq!(table.get(4), (5, true));
}

#[test]
fn test_table_delete() {
  let mut table = OpenIndexTable::new();
  table.insert(1, 2);
  table.insert(2, 3);
  table.insert(3, 4);
  table.insert(4, 5);
  assert_eq!(table.get(1), (2, true));
  assert_eq!(table.get(2), (3, true));
  assert_eq!(table.get(3), (4, true));
  assert_eq!(table.get(4), (5, true));
  assert_eq!(table.delete(1), (2, true));
  assert_eq!(table.delete(2), (3, true));
  assert_eq!(table.delete(3), (4, true));
  assert_eq!(table.delete(4), (5, true));
  assert_eq!(table.get(1), (0, false));
  assert_eq!(table.get(2), (0, false));
  assert_eq!(table.get(3), (0, false));
  assert_eq!(table.get(4), (0, false));
}

#[bench]
fn bench_std_map_insert(b: &mut Bencher) {
  let mut map = HashMap::new();
  let mut i: u64 = 0;
  b.iter(|| {
    map.insert(i, i);
    i += 1;
  });
}

#[bench]
fn bench_table_insert(b: &mut Bencher) {
  let mut table = OpenIndexTable::new();
  let mut i: u64 = 0;
  b.iter(|| {
    table.insert(i, i);
    i += 1;
  });
}

#[bench]
fn bench_std_map_get(b: &mut Bencher) {
  let mut map: HashMap<u64, u64> = HashMap::new();
  let max = 2 << 24;
  for i in 0..max {
    map.insert(i, i);
  }
  let mut i: u64 = 0;
  b.iter(|| {
    test::black_box(map.get(&i));
    i += 1;
    i %= max;
  });
}

#[bench]
fn bench_table_get(b: &mut Bencher) {
  let mut table = OpenIndexTable::new();
  let max = 2 << 24;
  for i in 0..max {
    table.insert(i, i);
  }
  let mut i: u64 = 0;
  b.iter(|| {
    test::black_box(table.get(i));
    i += 1;
    i %= max;
  });
}

#[bench]
fn bench_std_map_delete(b: &mut Bencher) {
  let mut map: HashMap<u64, u64> = HashMap::new();
  let max = 2 << 25;
  for i in 0..max {
    map.insert(i, i);
  }
  let mut i: u64 = 0;
  b.iter(|| {
    test::black_box(map.remove(&i));
    i += 1;
    if i % max == 0 {
      panic!("Benchmark to big")
    }
  });
}

#[bench]
fn bench_table_delete(b: &mut Bencher) {
  let mut table = OpenIndexTable::new();
  let max = 2 << 25;
  for i in 0..max {
    table.insert(i, i);
  }
  let mut i: u64 = 0;
  b.iter(|| {
    test::black_box(table.delete(i));
    i += 1;
    if i % max == 0 {
      panic!("Benchmark to big")
    }
  });
}
