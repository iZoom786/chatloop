//! SafeTensor format support with memory-mapped files
//!
//! This module provides zero-copy access to model weights stored in SafeTensor format.
//! All weights are memory-mapped for efficient access and minimal memory overhead.

use chatloop_common::{ChatLoopError, Result};
use memmap2::Mmap;
use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::fs::File;
use std::path::Path;
use std::sync::Arc;

/// Data type for SafeTensor
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum TensorDType {
    /// Float32
    F32,
    /// Float16
    F16,
    /// Int32
    I32,
    /// Int8
    I8,
    /// UInt8
    U8,
    /// Bool
    BOOL,
}

impl TensorDType {
    /// Get the size in bytes for this dtype
    pub fn size(&self) -> usize {
        match self {
            TensorDType::F32 => 4,
            TensorDType::F16 => 2,
            TensorDType::I32 => 4,
            TensorDType::I8 => 1,
            TensorDType::U8 => 1,
            TensorDType::BOOL => 1,
        }
    }

    /// Parse from string
    pub fn from_str(s: &str) -> Option<Self> {
        match s {
            "F32" => Some(TensorDType::F32),
            "F16" => Some(TensorDType::F16),
            "I32" => Some(TensorDType::I32),
            "I8" => Some(TensorDType::I8),
            "U8" => Some(TensorDType::U8),
            "BOOL" => Some(TensorDType::BOOL),
            _ => None,
        }
    }

    /// Convert to string
    pub fn as_str(&self) -> &'static str {
        match self {
            TensorDType::F32 => "F32",
            TensorDType::F16 => "F16",
            TensorDType::I32 => "I32",
            TensorDType::I8 => "I8",
            TensorDType::U8 => "U8",
            TensorDType::BOOL => "BOOL",
        }
    }
}

/// SafeTensor metadata header
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SafeTensorHeader {
    /// Map of tensor name to tensor info
    #[serde(rename = "tensors")]
    pub tensors: HashMap<String, TensorInfo>,
}

/// Information about a single tensor
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TensorInfo {
    /// Tensor dtype
    #[serde(rename = "dtype")]
    pub dtype: String,

    /// Tensor shape
    #[serde(rename = "shape")]
    pub shape: Vec<usize>,

    /// Data offsets (start, end) in bytes
    #[serde(rename = "data_offsets")]
    pub data_offsets: Vec<usize>,
}

impl TensorInfo {
    /// Get the dtype
    pub fn get_dtype(&self) -> Option<TensorDType> {
        TensorDType::from_str(&self.dtype)
    }

    /// Get the total size in bytes
    pub fn size_bytes(&self) -> usize {
        let dtype = self.get_dtype().unwrap_or(TensorDType::F32);
        let num_elements: usize = self.shape.iter().product();
        num_elements * dtype.size()
    }
}

/// Memory-mapped SafeTensor buffer
///
/// This provides zero-copy access to tensor data stored in SafeTensor format.
/// The memory is mapped directly from the file, avoiding loading the entire file into RAM.
pub struct SafeTensorBuffer {
    /// Memory-mapped file
    mmap: Mmap,

    /// Parsed header
    header: SafeTensorHeader,

    /// Length of header in bytes
    header_len: usize,
}

impl SafeTensorBuffer {
    /// Open a SafeTensor file with memory mapping
    ///
    /// This memory-maps the file, providing zero-copy access to tensor data.
    /// The OS manages paging, so only accessed portions are loaded into memory.
    pub fn open<P: AsRef<Path>>(path: P) -> Result<Self> {
        let path = path.as_ref();

        // Open the file
        let file = File::open(path)
            .map_err(|e| ChatLoopError::MemoryMap(format!("Failed to open file {}: {}", path.display(), e)))?;

        // Memory map the file
        let mmap = unsafe {
            Mmap::map(&file)
                .map_err(|e| ChatLoopError::MemoryMap(format!("Failed to mmap file {}: {}", path.display(), e)))?
        };

        // Parse the header (first 8 bytes is header length)
        if mmap.len() < 8 {
            return Err(ChatLoopError::MemoryMap("File too small to contain header".to_string()));
        }

        let header_len = u64::from_le_bytes([mmap[0], mmap[1], mmap[2], mmap[3], mmap[4], mmap[5], mmap[6], mmap[7]]) as usize;

        if mmap.len() < 8 + header_len {
            return Err(ChatLoopError::MemoryMap("File truncated: header length exceeds file size".to_string()));
        }

        // Parse JSON header
        let header_json = std::str::from_utf8(&mmap[8..8 + header_len])
            .map_err(|e| ChatLoopError::MemoryMap(format!("Invalid UTF-8 in header: {}", e)))?;

        let header: SafeTensorHeader = serde_json::from_str(header_json)
            .map_err(|e| ChatLoopError::MemoryMap(format!("Failed to parse header JSON: {}", e)))?;

        Ok(Self {
            mmap,
            header,
            header_len,
        })
    }

    /// Get the header
    pub fn header(&self) -> &SafeTensorHeader {
        &self.header
    }

    /// Get tensor names
    pub fn tensor_names(&self) -> impl Iterator<Item = &String> {
        self.header.tensors.keys()
    }

    /// Get a zero-copy view of a tensor
    ///
    /// This returns a view into the memory-mapped data without copying.
    pub fn get_tensor(&self, name: &str) -> Option<SafeTensorView<'_>> {
        let info = self.header.tensors.get(name)?;
        let data_start = 8 + self.header_len + info.data_offsets[0];
        let data_end = 8 + self.header_len + info.data_offsets[1];

        if data_end > self.mmap.len() {
            return None;
        }

        let dtype = info.get_dtype()?;
        let data = &self.mmap[data_start..data_end];

        Some(SafeTensorView {
            data,
            shape: info.shape.clone(),
            dtype,
        })
    }

    /// Get multiple tensors at once (more efficient)
    pub fn get_tensors(&self, names: &[&str]) -> HashMap<String, SafeTensorView<'_>> {
        let mut result = HashMap::new();

        for &name in names {
            if let Some(tensor) = self.get_tensor(name) {
                result.insert(name.to_string(), tensor);
            }
        }

        result
    }
}

/// Zero-copy view into a SafeTensor
///
/// This provides read-only access to tensor data without copying.
pub struct SafeTensorView<'a> {
    data: &'a [u8],
    shape: Vec<usize>,
    dtype: TensorDType,
}

impl<'a> SafeTensorView<'a> {
    /// Get the tensor shape
    pub fn shape(&self) -> &[usize] {
        &self.shape
    }

    /// Get the tensor dtype
    pub fn dtype(&self) -> TensorDType {
        self.dtype
    }

    /// Get the raw byte data
    pub fn data(&self) -> &'a [u8] {
        self.data
    }

    /// Interpret as f32 slice (if dtype matches)
    ///
    /// # Safety
    /// Caller must ensure dtype is F32
    pub unsafe fn as_f32_slice(&self) -> &[f32] {
        assert_eq!(self.dtype, TensorDType::F32);
        std::slice::from_raw_parts(self.data.as_ptr() as *const f32, self.data.len() / 4)
    }

    /// Interpret as f16 slice (if dtype matches)
    ///
    /// # Safety
    /// Caller must ensure dtype is F16
    pub unsafe fn as_f16_slice(&self) -> &[half::f16] {
        assert_eq!(self.dtype, TensorDType::F16);
        std::slice::from_raw_parts(self.data.as_ptr() as *const half::f16, self.data.len() / 2)
    }

    /// Get the number of elements
    pub fn len(&self) -> usize {
        self.shape.iter().product()
    }

    /// Check if the tensor is empty
    pub fn is_empty(&self) -> bool {
        self.len() == 0
    }
}

/// Thread-safe handle to SafeTensor buffer
///
/// This allows sharing the memory-mapped buffer across threads.
pub type SafeTensorRef = Arc<SafeTensorBuffer>;

#[cfg(test)]
mod tests {
    use super::*;
    use std::io::Write;
    use tempfile::NamedTempFile;

    fn create_test_safetensor() -> NamedTempFile {
        let header = json!({
            "tensors": {
                "weight": {
                    "dtype": "F32",
                    "shape": [2, 2],
                    "data_offsets": [0, 16]
                }
            }
        });

        let header_json = serde_json::to_string(&header).unwrap();
        let header_len = header_json.len() as u64;

        let mut file = NamedTempFile::new().unwrap();
        file.write_all(&header_len.to_le_bytes()).unwrap();
        file.write_all(header_json.as_bytes()).unwrap();

        // Write tensor data (4 floats = 16 bytes)
        let data: [u8; 16] = [
            0, 0, 128, 63,  // 1.0
            0, 0, 0, 64,    // 2.0
            0, 0, 64, 64,   // 3.0
            0, 0, 128, 64,  // 4.0
        ];
        file.write_all(&data).unwrap();

        file
    }

    #[test]
    fn test_safetensor_open() {
        let file = create_test_safetensor();
        let buffer = SafeTensorBuffer::open(file.path()).unwrap();

        assert_eq!(buffer.tensor_names().count(), 1);

        let tensor = buffer.get_tensor("weight").unwrap();
        assert_eq!(tensor.shape(), vec![2, 2]);
        assert_eq!(tensor.dtype(), TensorDType::F32);
        assert_eq!(tensor.len(), 4);

        let data = unsafe { tensor.as_f32_slice() };
        assert_eq!(data, &[1.0, 2.0, 3.0, 4.0]);
    }
}
