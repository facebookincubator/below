// Copyright (c) Facebook, Inc. and its affiliates.
//
// Licensed under the Apache License, Version 2.0 (the "License");
// you may not use this file except in compliance with the License.
// You may obtain a copy of the License at
//
//     http://www.apache.org/licenses/LICENSE-2.0
//
// Unless required by applicable law or agreed to in writing, software
// distributed under the License is distributed on an "AS IS" BASIS,
// WITHOUT WARRANTIES OR CONDITIONS OF ANY KIND, either express or implied.
// See the License for the specific language governing permissions and
// limitations under the License.

use anyhow::Result;

use crate::Direction;

/// A read-only Iterator that can move back and forth.
pub trait Cursor {
    type Offset;
    type Item;

    /// Get offset of the cursor.
    fn get_offset(&self) -> Self::Offset;
    /// Set offset of the cursor.
    fn set_offset(&mut self, offset: Self::Offset);
    /// Get the item pointed to by the cursor. Could be None if item is invalid.
    fn get(&self) -> Option<Self::Item>;
    /// Move the cursor one step in the given direction if it's within range.
    /// Return if the cursor is moved.
    fn advance(&mut self, direction: Direction) -> Result<bool>;
    /// Move the cursor in the given direction until a valid item is obtained.
    /// If no more valid item available, return None and offset unchanged.
    fn next(&mut self, direction: Direction) -> Result<Option<Self::Item>> {
        let offset = self.get_offset();
        while self.advance(direction)? {
            if let Some(item) = self.get() {
                return Ok(Some(item));
            }
        }
        self.set_offset(offset);
        Ok(None)
    }
}

/// A cursor that has keys bound to each position in ascending order.
/// Making this generic so that each Cursor may define multiple Keys.
pub trait KeyedCursor<Key: std::cmp::Ord>: Cursor {
    /// Get the key of the current position.
    fn get_key(&self) -> Option<Key>;

    /// Used by jump_to_keys with same parameters. Should be overridden to make
    /// jump_to_key more efficient by moving cursor near the key. Notice that
    /// jump_to_key should work as long as keys are monotonic in same order as
    /// offsets, even if this function does nothing.
    fn jump_near_key(&mut self, _key: &Key, _direction: Direction) {}

    /// Move the cursor to the closest valid pos to the given key in the given
    /// direction. Return if the key at the final pos satisfies the ordering
    /// requirement. For example, for a list of int keys: [3, 7, 8, 12, 19],
    /// jumping to key 9 forward moves cursor to index 3 because 12 is the
    /// smallest int greater than 9 in the list. Jumping to key 0 reverse moves
    /// cursor to index 0 and returns false because no key is smaller than 3.
    fn jump_to_key(&mut self, key: &Key, direction: Direction) -> Result<bool> {
        self.jump_near_key(key, direction);
        // Move cursor backward to get a position with lower key order, and then
        // move forward to get the first position with higher key order.
        let mut curr_key = self.get_key();
        for curr_dir in &[direction.flip(), direction] {
            let skip_order = curr_dir.get_skip_order();
            while curr_key.as_ref().map_or(true, |k| k.cmp(key) == skip_order) {
                if !self.advance(*curr_dir)? {
                    break;
                }
                curr_key = self.get_key();
            }
        }
        // Check if the last key satisfies the direction order
        Ok(curr_key.map_or(false, |k| k.cmp(key) != direction.get_skip_order()))
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use Direction::{Forward, Reverse};

    /// Simple cursor to illustrate implementation and test default methods.
    struct TestCursor<'a> {
        data: &'a Vec<Option<i32>>,
        offset: Option<usize>,
    }
    impl Cursor for TestCursor<'_> {
        type Offset = Option<usize>;
        type Item = i32;
        fn get_offset(&self) -> Self::Offset {
            self.offset
        }
        fn set_offset(&mut self, offset: Self::Offset) {
            self.offset = offset;
        }
        fn get(&self) -> Option<Self::Item> {
            self.offset
                .as_ref()
                .and_then(|o| self.data.get(*o).cloned().flatten())
        }
        fn advance(&mut self, direction: Direction) -> Result<bool> {
            let offset = match self.offset {
                Some(offset) => match direction {
                    Direction::Forward => offset.checked_add(1).filter(|o| o < &self.data.len()),
                    Direction::Reverse => offset.checked_sub(1),
                },
                // Default offsets
                None => match direction {
                    Direction::Forward => Some(0).filter(|o| o < &self.data.len()),
                    Direction::Reverse => self.data.len().checked_sub(1),
                },
            };
            if offset.is_some() {
                self.offset = offset;
                Ok(true)
            } else {
                Ok(false)
            }
        }
    }
    impl KeyedCursor<i32> for TestCursor<'_> {
        fn get_key(&self) -> Option<i32> {
            self.get()
        }
    }

    /// Test default implementation of next(). It should skip invalid items.
    #[test]
    fn default_next() {
        let data = vec![None, Some(3), Some(5), None, None, Some(9)];
        let mut cursor = TestCursor {
            data: &data,
            offset: None,
        };
        assert_eq!(cursor.next(Forward).unwrap(), Some(3));
        assert_eq!(cursor.next(Forward).unwrap(), Some(5));
        assert_eq!(cursor.next(Forward).unwrap(), Some(9));
        assert_eq!(cursor.next(Forward).unwrap(), None);
        assert_eq!(cursor.next(Reverse).unwrap(), Some(5));
        assert_eq!(cursor.next(Reverse).unwrap(), Some(3));
        assert_eq!(cursor.next(Reverse).unwrap(), None);
        // Offset unchanged after reaching boundry
        assert_eq!(cursor.get(), Some(3));
    }

    /// Test default implementation of jump_to_key().
    #[test]
    fn default_jump_to_key() {
        let data = vec![None, Some(3), Some(5), None, None, Some(9)];
        let mut cursor = TestCursor {
            data: &data,
            offset: None,
        };
        // Exact key
        assert!(cursor.jump_to_key(&3, Forward).unwrap());
        assert_eq!(cursor.get_key(), Some(3));
        assert!(cursor.jump_to_key(&5, Reverse).unwrap());
        assert_eq!(cursor.get_key(), Some(5));
        // Closest key
        assert!(cursor.jump_to_key(&7, Forward).unwrap());
        assert_eq!(cursor.get_key(), Some(9));
        assert!(cursor.jump_to_key(&4, Reverse).unwrap());
        assert_eq!(cursor.get_key(), Some(3));
        // No key satisfies direction constrain.
        assert!(!cursor.jump_to_key(&10, Forward).unwrap());
        assert_eq!(cursor.get_key(), Some(9));
        assert!(!cursor.jump_to_key(&0, Reverse).unwrap());
        assert_eq!(cursor.get_key(), None);
    }
}
