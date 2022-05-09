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

use std::ops::{Deref, DerefMut};

#[repr(C)]
pub struct WithMemAfter<T, const N: usize> {
    value: T,
    extra: [u8; N],
}

impl<T: Sized, const N: usize> WithMemAfter<T, N> {
    pub fn new() -> Self {
        unsafe {
            WithMemAfter {
                value: std::mem::zeroed(),
                extra: [0; N],
            }
        }
    }

    pub fn as_mut_ptr(&mut self) -> *mut T {
        &mut self.value
    }

    pub fn total_size(&self) -> usize {
        std::mem::size_of::<Self>()
    }

    pub fn extra_ptr(&self) -> *const u8 {
        self.extra.as_ptr()
    }

    pub fn extra_size(&self) -> usize {
        N
    }
}

impl<T: Sized, const N: usize> Deref for WithMemAfter<T, N> {
    type Target = T;

    fn deref(&self) -> &Self::Target {
        &self.value
    }
}

impl<T: Sized, const N: usize> DerefMut for WithMemAfter<T, N> {
    fn deref_mut(&mut self) -> &mut Self::Target {
        &mut self.value
    }
}

pub unsafe fn get_and_move(ptr: &mut *const u8, n: usize) -> *const u8 {
    let res = *ptr;
    *ptr = (*ptr).add(n);
    res
}

pub unsafe fn get_and_move_typed<T: Sized>(ptr: &mut *const u8) -> *const T {
    get_and_move(ptr, std::mem::size_of::<T>()) as *const T
}
