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

use anyhow::{anyhow, bail, Context, Error, Result};
use bytes::Bytes;

/// This file defines a minimalistic compressor and decompressor interface
/// optimized for below's usage. They are wrappers around general compression
/// libraries. Currently only zstd is supported.

// TODO: Use latest zstd as implementation
// TODO: Consider using experimental feature to load dict by reference

fn code_to_err(code: zstd_safe::ErrorCode) -> Error {
    anyhow!(zstd_safe::get_error_name(code))
}

pub struct Compressor {
    cctx: zstd_safe::CCtx<'static>,
    dict_loaded: bool,
}

impl Compressor {
    pub fn new() -> Self {
        Self {
            cctx: zstd_safe::CCtx::create(),
            dict_loaded: false,
        }
    }

    /// Resets the dict loaded.
    fn reset_dict(&mut self) -> Result<()> {
        if self.dict_loaded {
            self.cctx
                .load_dictionary(&[])
                .map_err(code_to_err)
                .context("Failed to load empty dictionary")?;
            self.dict_loaded = false;
        }
        Ok(())
    }

    /// Loads the given dict.
    pub fn load_dict(&mut self, dict: &[u8]) -> Result<()> {
        self.cctx
            .load_dictionary(dict)
            .map_err(code_to_err)
            .context("Failed to load dictionary")?;
        self.dict_loaded = true;
        Ok(())
    }

    /// Compresses the given frame using the previously loaded dict, if any.
    pub fn compress_with_loaded_dict(&mut self, frame: &[u8]) -> Result<Bytes> {
        let mut buf = Vec::with_capacity(zstd_safe::compress_bound(frame.len()));
        self.cctx
            .compress2(&mut buf, frame)
            .map_err(code_to_err)
            .context("zstd compress2 failed")?;
        Ok(buf.into())
    }

    /// Compresses the given frame after resetting dict.
    pub fn compress_with_dict_reset(&mut self, frame: &[u8]) -> Result<Bytes> {
        self.reset_dict().context("Failed to reload dict")?;
        self.compress_with_loaded_dict(frame)
            .context("Failed to compress without dict")
    }
}

pub struct Decompressor<K> {
    dctx: zstd_safe::DCtx<'static>,
    dict: Bytes,
    dict_key: Option<K>,
}

impl<K> Decompressor<K> {
    pub fn new() -> Self {
        Self {
            dctx: zstd_safe::DCtx::create(),
            dict: Bytes::new(),
            dict_key: None,
        }
    }

    /// Gets the dict which is also the decompressed key frame.
    pub fn get_dict(&self) -> &Bytes {
        &self.dict
    }

    /// Gets the key associated with the loaded dict.
    pub fn get_dict_key(&self) -> Option<&K> {
        self.dict_key.as_ref()
    }

    /// Resets the dict loaded to dctx.
    fn reset_dict(&mut self) -> Result<()> {
        if !self.dict.is_empty() {
            self.dctx
                .load_dictionary(&[])
                .map_err(code_to_err)
                .context("Failed to load empty dictionary")?;
            self.dict = Bytes::new();
            self.dict_key = None;
        }
        Ok(())
    }

    /// Loads the given dict and associates it with the given key, whose meaning
    /// is user-defined. Only frames with a matching key should be decompressed
    /// with this dict.
    pub fn load_dict(&mut self, dict: Bytes, key: K) -> Result<()> {
        self.dctx
            .load_dictionary(&dict)
            .map_err(code_to_err)
            .context("Failed to load zstd dictionary by reference")?;
        self.dict = dict;
        self.dict_key = Some(key);
        Ok(())
    }

    /// Decompresses the given frame using the previously loaded dict, if any.
    pub fn decompress_with_loaded_dict(&mut self, frame: &[u8]) -> Result<Bytes> {
        let capacity = match zstd_safe::get_frame_content_size(frame) {
            zstd_safe::CONTENTSIZE_ERROR => bail!("Error getting frame content size"),
            // Decompressed size should only be unknown when using streaming
            // mode, which we should never use
            zstd_safe::CONTENTSIZE_UNKNOWN => bail!("Unknown decompressed size"),
            capacity => capacity as usize,
        };
        let mut buf = Vec::with_capacity(capacity);
        self.dctx
            .decompress(&mut buf, frame)
            .map_err(code_to_err)
            .context("zstd decompress failed")?;
        Ok(buf.into())
    }

    /// Decompresses the given frame after resetting dict.
    pub fn decompress_with_dict_reset(&mut self, frame: &[u8]) -> Result<Bytes> {
        self.reset_dict().context("Failed to reload dict")?;
        self.decompress_with_loaded_dict(frame)
            .context("Failed to decompress without dict")
    }
}

#[cfg(test)]
mod test {
    use super::*;

    fn gen_data(n: usize) -> Vec<u8> {
        use std::hash::Hasher;

        let mut data = Vec::with_capacity(n);
        let mut hasher = std::collections::hash_map::DefaultHasher::new();
        hasher.write_u64(0xfaceb00c);
        while data.len() < n {
            let val = hasher.finish();
            data.extend(val.to_be_bytes());
            hasher.write_u64(val);
        }
        data
    }

    #[test]
    fn compressor_decompressor() {
        let mut c = Compressor::new();
        let mut d = Decompressor::new();

        let data: Bytes = gen_data(128).into();

        let comp_default = c
            .compress_with_loaded_dict(&data)
            .expect("Failed to compress");

        c.load_dict(&data).expect("Fail to load dict");
        let comp_with_dict = c
            .compress_with_loaded_dict(&data)
            .expect("Failed to compress");

        let comp_dict_reset = c
            .compress_with_dict_reset(&data)
            .expect("Failed to compress");

        // Using self as dict should get much smaller result than without dict
        assert!(comp_with_dict.len() < comp_default.len());
        // Compress with dict reset should be the same as with default dict
        assert_eq!(comp_dict_reset, comp_default);

        let decomp_default = d
            .decompress_with_loaded_dict(&comp_default)
            .expect("Failed to decompress");

        d.load_dict(data.clone(), ()).expect("Failed to load dict");
        let decomp_with_dict = d
            .decompress_with_loaded_dict(&comp_with_dict)
            .expect("Failed to decompress");

        let decomp_dict_reset = d
            .decompress_with_dict_reset(&comp_dict_reset)
            .expect("Failed to decompress");

        // All should be decompressed back to original data
        assert_eq!(decomp_default, data);
        assert_eq!(decomp_with_dict, data);
        assert_eq!(decomp_dict_reset, data);
    }

    #[test]
    fn compatibility() {
        let data: Bytes = gen_data(128).into();

        // Compressor => zstd decompress
        {
            let comp = Compressor::new()
                .compress_with_dict_reset(&data)
                .expect("Failed to compress");
            let decomp = zstd::stream::decode_all(&*comp).expect("Failed to decompress");
            assert_eq!(decomp, data);
        }
        // zstd compress => Decompressor
        {
            let comp = zstd::block::compress(&data, 0).expect("Failed to compress");
            let decomp = Decompressor::<()>::new()
                .decompress_with_dict_reset(&comp)
                .expect("Failed to decompress");
            assert_eq!(decomp, data);
        }
    }
}
