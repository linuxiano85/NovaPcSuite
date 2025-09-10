// Copyright 2025 linuxiano85
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

//! Nova Formats - Contact export formats and data structure definitions

pub mod contacts;
pub mod exporters;

pub use contacts::{Contact, ContactSource, PhoneNumber};
pub use exporters::{ContactExporter, CsvExporter, ExportFormat, VcfExporter};
