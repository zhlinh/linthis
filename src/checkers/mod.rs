// Copyright 2024 zhlinh and linthis Project Authors. All rights reserved.
// Use of this source code is governed by a MIT-style
// license that can be found at
//
// https://opensource.org/license/MIT
//
// The above copyright notice and this permission
// notice shall be included in all copies or
// substantial portions of the Software.

//! Language-specific linter implementations.

pub mod cpp;
pub mod go;
pub mod java;
pub mod python;
pub mod rust;
pub mod traits;
pub mod typescript;

pub use cpp::CppChecker;
pub use go::GoChecker;
pub use java::JavaChecker;
pub use python::PythonChecker;
pub use rust::RustChecker;
pub use traits::Checker;
pub use typescript::TypeScriptChecker;
