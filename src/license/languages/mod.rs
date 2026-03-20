// Copyright 2024 zhlinh and linthis Project Authors. All rights reserved.
// Use of this source code is governed by a MIT-style
// license that can be found at
//
// https://opensource.org/license/MIT
//
// The above copyright notice and this permission
// notice shall be included in all copies or
// substantial portions of the Software.

//! Language-specific license scanners.

mod go;
mod java;
mod node;
mod python;
mod rust;

pub use go::GoLicenseScanner;
pub use java::JavaLicenseScanner;
pub use node::NodeLicenseScanner;
pub use python::PythonLicenseScanner;
pub use rust::RustLicenseScanner;
