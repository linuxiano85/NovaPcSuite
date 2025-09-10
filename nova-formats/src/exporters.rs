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

use crate::contacts::Contact;
use nova_core::Result;
use std::path::{Path, PathBuf};
use tracing::info;

#[derive(Debug, Clone)]
pub enum ExportFormat {
    Vcf,
    Csv,
}

#[async_trait::async_trait]
pub trait ContactExporter {
    async fn export_contacts(&self, contacts: &[Contact], output_path: &Path) -> Result<()>;
    fn format(&self) -> ExportFormat;
}

pub struct VcfExporter;

impl Default for & {
    fn default() -> Self {
        Self::new()
    }
}

impl Default for VcfExporter {
    fn default() -> Self {
        Self::new()
    }
}

impl Default for & {
    fn default() -> Self {
        Self::new()
    }
}

impl VcfExporter {
    pub fn new() -> Self {
        Self
    }
}

#[async_trait::async_trait]
impl Default for & {
    fn default() -> Self {
        Self::new()
    }
}

impl ContactExporter for VcfExporter {
    async fn export_contacts(&self, contacts: &[Contact], output_path: &PathBuf) -> Result<()> {
        info!(
            "Exporting {} contacts to VCF format: {:?}",
            contacts.len(),
            output_path
        );

        let mut vcf_content = String::new();

        for contact in contacts {
            vcf_content.push_str(&self.contact_to_vcf(contact));
            vcf_content.push('\n');
        }

        std::fs::write(output_path, vcf_content)?;

        info!("Successfully exported {} contacts to VCF", contacts.len());
        Ok(())
    }

    fn format(&self) -> ExportFormat {
        ExportFormat::Vcf
    }
}

impl Default for & {
    fn default() -> Self {
        Self::new()
    }
}

impl VcfExporter {
    fn contact_to_vcf(&self, contact: &Contact) -> String {
        let mut vcf = String::new();

        vcf.push_str("BEGIN:VCARD\n");
        vcf.push_str("VERSION:3.0\n");

        // Full name
        vcf.push_str(&format!(
            "FN:{}\n",
            self.escape_vcf_value(&contact.display_name)
        ));

        // Structured name (Family;Given;Middle;Prefix;Suffix)
        let family = contact.family_name.as_deref().unwrap_or("");
        let given = contact.given_name.as_deref().unwrap_or("");
        vcf.push_str(&format!(
            "N:{};{};;;\n",
            self.escape_vcf_value(family),
            self.escape_vcf_value(given)
        ));

        // Phone numbers
        for phone in &contact.phone_numbers {
            let type_param = match phone.type_.as_str() {
                "mobile" => "CELL",
                "home" => "HOME",
                "work" => "WORK",
                _ => "VOICE",
            };
            vcf.push_str(&format!("TEL;TYPE={}:{}\n", type_param, phone.number));
        }

        // Email addresses
        for email in &contact.email_addresses {
            vcf.push_str(&format!("EMAIL:{}\n", self.escape_vcf_value(email)));
        }

        // Organization
        if let Some(ref org) = contact.organization {
            vcf.push_str(&format!("ORG:{}\n", self.escape_vcf_value(org)));
        }

        // Note
        if let Some(ref note) = contact.note {
            vcf.push_str(&format!("NOTE:{}\n", self.escape_vcf_value(note)));
        }

        vcf.push_str("END:VCARD\n");

        vcf
    }

    fn escape_vcf_value(&self, value: &str) -> String {
        value
            .replace('\n', "\\n")
            .replace('\r', "\\r")
            .replace(',', "\\,")
            .replace(';', "\\;")
            .replace('\\', "\\\\")
    }
}

pub struct CsvExporter;

impl Default for & {
    fn default() -> Self {
        Self::new()
    }
}

impl CsvExporter {
    pub fn new() -> Self {
        Self
    }
}

#[async_trait::async_trait]
impl Default for & {
    fn default() -> Self {
        Self::new()
    }
}

impl ContactExporter for CsvExporter {
    async fn export_contacts(&self, contacts: &[Contact], output_path: &PathBuf) -> Result<()> {
        info!(
            "Exporting {} contacts to CSV format: {:?}",
            contacts.len(),
            output_path
        );

        let mut csv_content = String::new();

        // CSV header
        csv_content.push_str("id,display_name,given_name,family_name,phone_numbers,email_addresses,organization,note\n");

        for contact in contacts {
            csv_content.push_str(&self.contact_to_csv_row(contact));
            csv_content.push('\n');
        }

        std::fs::write(output_path, csv_content)?;

        info!("Successfully exported {} contacts to CSV", contacts.len());
        Ok(())
    }

    fn format(&self) -> ExportFormat {
        ExportFormat::Csv
    }
}

impl Default for & {
    fn default() -> Self {
        Self::new()
    }
}

impl CsvExporter {
    fn contact_to_csv_row(&self, contact: &Contact) -> String {
        let phone_numbers = contact
            .phone_numbers
            .iter()
            .map(|p| p.number.clone())
            .collect::<Vec<_>>()
            .join("|");

        let email_addresses = contact.email_addresses.join("|");

        format!(
            "{},{},{},{},{},{},{},{}",
            self.escape_csv_value(&contact.id),
            self.escape_csv_value(&contact.display_name),
            self.escape_csv_value(&contact.given_name.as_deref().unwrap_or("")),
            self.escape_csv_value(&contact.family_name.as_deref().unwrap_or("")),
            self.escape_csv_value(&phone_numbers),
            self.escape_csv_value(&email_addresses),
            self.escape_csv_value(&contact.organization.as_deref().unwrap_or("")),
            self.escape_csv_value(&contact.note.as_deref().unwrap_or(""))
        )
    }

    fn escape_csv_value(&self, value: &str) -> String {
        if value.contains(',') || value.contains('"') || value.contains('\n') {
            format!("\"{}\"", value.replace('"', "\"\""))
        } else {
            value.to_string()
        }
    }
}
