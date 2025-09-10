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

use nova_adb::AdbClient;
use nova_core::{Device, Result};
use serde::{Deserialize, Serialize};
use tracing::{debug, info, warn};

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Contact {
    pub id: String,
    pub display_name: String,
    pub given_name: Option<String>,
    pub family_name: Option<String>,
    pub phone_numbers: Vec<PhoneNumber>,
    pub email_addresses: Vec<String>,
    pub organization: Option<String>,
    pub note: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PhoneNumber {
    pub number: String,
    pub type_: String, // "mobile", "home", "work", etc.
    pub label: Option<String>,
}

#[async_trait::async_trait]
pub trait ContactSource {
    async fn fetch_contacts(&self, device: &Device) -> Result<Vec<Contact>>;
    async fn get_contact_count(&self, device: &Device) -> Result<usize>;
}

pub struct AndroidContactSource {
    adb_client: AdbClient,
}

impl Default for & {
    fn default() -> Self {
        Self::new()
    }
}

impl AndroidContactSource {
    pub fn new() -> Self {
        Self {
            adb_client: AdbClient::new(),
        }
    }
}

#[async_trait::async_trait]
impl Default for & {
    fn default() -> Self {
        Self::new()
    }
}

impl ContactSource for AndroidContactSource {
    async fn fetch_contacts(&self, device: &Device) -> Result<Vec<Contact>> {
        info!("Fetching contacts from device: {}", device.info.serial);

        // Query contacts using content provider
        let query_cmd = "content query --uri content://com.android.contacts/contacts";
        let output = self
            .adb_client
            .shell_command(&device.info.serial, query_cmd)
            .await?;

        debug!("Raw contacts query output: {}", output);

        // For now, return mock contacts since parsing the actual output is complex
        // TODO: Implement proper content provider response parsing
        warn!("Contact parsing not yet fully implemented, returning mock data");

        let mock_contacts = vec![
            Contact {
                id: "1".to_string(),
                display_name: "John Doe".to_string(),
                given_name: Some("John".to_string()),
                family_name: Some("Doe".to_string()),
                phone_numbers: vec![PhoneNumber {
                    number: "+1234567890".to_string(),
                    type_: "mobile".to_string(),
                    label: None,
                }],
                email_addresses: vec!["john.doe@example.com".to_string()],
                organization: Some("Example Corp".to_string()),
                note: None,
            },
            Contact {
                id: "2".to_string(),
                display_name: "Jane Smith".to_string(),
                given_name: Some("Jane".to_string()),
                family_name: Some("Smith".to_string()),
                phone_numbers: vec![PhoneNumber {
                    number: "+0987654321".to_string(),
                    type_: "home".to_string(),
                    label: None,
                }],
                email_addresses: vec!["jane.smith@example.com".to_string()],
                organization: None,
                note: Some("Important contact".to_string()),
            },
        ];

        info!("Retrieved {} contacts", mock_contacts.len());
        Ok(mock_contacts)
    }

    async fn get_contact_count(&self, device: &Device) -> Result<usize> {
        debug!("Getting contact count for device: {}", device.info.serial);

        // Query contact count
        let count_cmd = "content query --uri content://com.android.contacts/contacts | wc -l";
        let output = self
            .adb_client
            .shell_command(&device.info.serial, count_cmd)
            .await?;

        let count = output.trim().parse::<usize>().unwrap_or(0);
        debug!("Found {} contacts", count);

        Ok(count)
    }
}
