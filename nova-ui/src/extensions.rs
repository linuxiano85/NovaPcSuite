use eframe::egui;
use nova_plugin_api::{PluginRegistry, PluginDescriptor, PluginHealth};
use std::sync::Arc;
use std::collections::HashMap;

/// Extensions/Plugins management UI
pub struct ExtensionsUI {
    plugin_registry: Arc<PluginRegistry>,
    plugins: Vec<PluginDescriptor>,
    plugin_health: HashMap<String, PluginHealth>,
    selected_plugin: Option<String>,
    refresh_requested: bool,
}

impl ExtensionsUI {
    pub fn new(plugin_registry: Arc<PluginRegistry>) -> Self {
        Self {
            plugin_registry,
            plugins: vec![],
            plugin_health: HashMap::new(),
            selected_plugin: None,
            refresh_requested: true,
        }
    }

    pub fn update(&mut self, ui: &mut egui::Ui, _ctx: &egui::Context) {
        ui.heading("Extensions");
        ui.separator();

        // Refresh button
        ui.horizontal(|ui| {
            if ui.button("🔄 Refresh").clicked() {
                self.refresh_requested = true;
            }
            ui.separator();
            ui.label(format!("Total plugins: {}", self.plugins.len()));
        });

        ui.separator();

        // Handle refresh request (async operations need to be handled in the main app loop)
        if self.refresh_requested {
            self.refresh_requested = false;
            // In a real implementation, this would trigger an async refresh
            // For now, we'll show placeholder data
            self.plugins = vec![
                create_example_plugin_descriptor("backup-analyzer", "Backup Analyzer", "Analyzes backup efficiency"),
                create_example_plugin_descriptor("cloud-sync", "Cloud Sync", "Synchronizes data to cloud storage"),
            ];
            self.plugin_health.insert("backup-analyzer".to_string(), PluginHealth::Healthy);
            self.plugin_health.insert("cloud-sync".to_string(), PluginHealth::Warning { 
                message: "Configuration needed".to_string() 
            });
        }

        // Two-column layout
        ui.horizontal(|ui| {
            // Left panel: Plugin list
            ui.allocate_ui_with_layout(
                [ui.available_width() * 0.4, ui.available_height()].into(),
                egui::Layout::top_down(egui::Align::LEFT),
                |ui| {
                    ui.heading("Installed Plugins");
                    ui.separator();

                    egui::ScrollArea::vertical().show(ui, |ui| {
                        for plugin in &self.plugins {
                            let is_selected = self.selected_plugin.as_ref() == Some(&plugin.id);
                            let health = self.plugin_health.get(&plugin.id);
                            
                            let status_icon = match health {
                                Some(PluginHealth::Healthy) => "✅",
                                Some(PluginHealth::Warning { .. }) => "⚠️",
                                Some(PluginHealth::Error { .. }) => "❌",
                                None => "❓",
                            };

                            if ui.selectable_label(is_selected, format!("{} {}", status_icon, plugin.name)).clicked() {
                                self.selected_plugin = Some(plugin.id.clone());
                            }
                        }
                    });
                },
            );

            ui.separator();

            // Right panel: Plugin details
            ui.allocate_ui_with_layout(
                [ui.available_width(), ui.available_height()].into(),
                egui::Layout::top_down(egui::Align::LEFT),
                |ui| {
                    if let Some(selected_id) = &self.selected_plugin {
                        if let Some(plugin) = self.plugins.iter().find(|p| &p.id == selected_id) {
                            let plugin_clone = plugin.clone();
                            self.show_plugin_details(ui, &plugin_clone);
                        }
                    } else {
                        ui.heading("Plugin Details");
                        ui.separator();
                        ui.label("Select a plugin to view details");
                    }
                },
            );
        });
    }

    fn show_plugin_details(&mut self, ui: &mut egui::Ui, plugin: &PluginDescriptor) {
        ui.heading(&plugin.name);
        ui.separator();

        // Basic info
        ui.horizontal(|ui| {
            ui.label("Version:");
            ui.code(plugin.version.to_string());
        });

        ui.horizontal(|ui| {
            ui.label("ID:");
            ui.code(&plugin.id);
        });

        ui.horizontal(|ui| {
            ui.label("API Version:");
            ui.code(plugin.api_version.to_string());
        });

        ui.add_space(10.0);

        // Description
        ui.label("Description:");
        ui.label(&plugin.description);

        ui.add_space(10.0);

        // Authors
        if !plugin.authors.is_empty() {
            ui.label("Authors:");
            for author in &plugin.authors {
                ui.label(format!("  • {}", author));
            }
            ui.add_space(10.0);
        }

        // Categories
        if !plugin.categories.is_empty() {
            ui.label("Categories:");
            ui.horizontal_wrapped(|ui| {
                for category in &plugin.categories {
                    ui.label(egui::RichText::new(format!("{:?}", category)).weak());
                }
            });
            ui.add_space(10.0);
        }

        // Health status
        if let Some(health) = self.plugin_health.get(&plugin.id) {
            ui.label("Status:");
            match health {
                PluginHealth::Healthy => {
                    ui.colored_label(egui::Color32::GREEN, "✅ Healthy");
                }
                PluginHealth::Warning { message } => {
                    ui.colored_label(egui::Color32::YELLOW, format!("⚠️ Warning: {}", message));
                }
                PluginHealth::Error { message } => {
                    ui.colored_label(egui::Color32::RED, format!("❌ Error: {}", message));
                }
            }
            ui.add_space(10.0);
        }

        // Capabilities
        ui.label("Capabilities:");
        ui.indent("capabilities", |ui| {
            capability_checkbox(ui, "File System Access", plugin.capabilities.file_system_access);
            capability_checkbox(ui, "Network Access", plugin.capabilities.network_access);
            capability_checkbox(ui, "System Info Access", plugin.capabilities.system_info_access);
            capability_checkbox(ui, "Backup Events", plugin.capabilities.backup_events);
            capability_checkbox(ui, "UI Panels", plugin.capabilities.ui_panels);
            capability_checkbox(ui, "Config UI", plugin.capabilities.config_ui);
        });

        ui.add_space(10.0);

        // Action buttons
        ui.horizontal(|ui| {
            if ui.button("Configure").clicked() {
                // TODO: Open plugin configuration dialog
            }
            if ui.button("Disable").clicked() {
                // TODO: Disable plugin
            }
            if ui.button("Remove").clicked() {
                // TODO: Remove plugin
            }
        });
    }
}

fn capability_checkbox(ui: &mut egui::Ui, label: &str, enabled: bool) {
    ui.horizontal(|ui| {
        ui.checkbox(&mut enabled.clone(), "");
        ui.label(label);
    });
}

fn create_example_plugin_descriptor(id: &str, name: &str, description: &str) -> PluginDescriptor {
    PluginDescriptor {
        id: id.to_string(),
        name: name.to_string(),
        version: semver::Version::new(1, 0, 0),
        api_version: nova_plugin_api::CURRENT_API_VERSION,
        authors: vec!["Example Author".to_string()],
        description: description.to_string(),
        categories: vec![nova_plugin_api::PluginCategory::Backup],
        capabilities: nova_plugin_api::PluginCapabilities {
            file_system_access: id == "backup-analyzer",
            network_access: id == "cloud-sync",
            backup_events: true,
            ..Default::default()
        },
        dependencies: std::collections::HashMap::new(),
        entry_point: None,
    }
}