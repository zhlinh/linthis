// Copyright 2024 zhlinh and linthis Project Authors. All rights reserved.
// Use of this source code is governed by a MIT-style
// license that can be found at
//
// https://opensource.org/license/MIT
//
// The above copyright notice and this permission
// notice shall be included in all copies or
// substantial portions of the Software.

//! Language-specific security scanners.

mod rust;
mod node;
mod python;
mod go;
mod java;

pub use rust::RustSecurityScanner;
pub use node::NodeSecurityScanner;
pub use python::PythonSecurityScanner;
pub use go::GoSecurityScanner;
pub use java::JavaSecurityScanner;
