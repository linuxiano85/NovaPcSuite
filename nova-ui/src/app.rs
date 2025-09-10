use eframe::egui;
use nova_plugin_api::PluginRegistry;
use std::sync::Arc;

/// Main application UI
pub struct NovaApp {
    plugin_registry: Arc<PluginRegistry>,
    current_tab: AppTab,
    extensions_ui: crate::extensions::ExtensionsUI,
}

#[derive(Debug, Clone, PartialEq)]
enum AppTab {
    Dashboard,
    Backup,
    Extensions,
    Settings,
}

impl NovaApp {
    pub fn new(plugin_registry: Arc<PluginRegistry>) -> Self {
        Self {
            plugin_registry: plugin_registry.clone(),
            current_tab: AppTab::Dashboard,
            extensions_ui: crate::extensions::ExtensionsUI::new(plugin_registry),
        }
    }
}

impl eframe::App for NovaApp {
    fn update(&mut self, ctx: &egui::Context, _frame: &mut eframe::Frame) {
        // Top menu bar
        egui::TopBottomPanel::top("top_panel").show(ctx, |ui| {
            egui::menu::bar(ui, |ui| {
                ui.menu_button("File", |ui| {
                    if ui.button("Exit").clicked() {
                        ctx.send_viewport_cmd(egui::ViewportCommand::Close);
                    }
                });
                ui.menu_button("Help", |ui| {
                    if ui.button("About").clicked() {
                        // Show about dialog
                    }
                });
            });
        });

        // Side panel for navigation
        egui::SidePanel::left("side_panel").show(ctx, |ui| {
            ui.heading("NovaPcSuite");
            ui.separator();

            if ui.selectable_label(self.current_tab == AppTab::Dashboard, "📊 Dashboard").clicked() {
                self.current_tab = AppTab::Dashboard;
            }
            if ui.selectable_label(self.current_tab == AppTab::Backup, "💾 Backup").clicked() {
                self.current_tab = AppTab::Backup;
            }
            if ui.selectable_label(self.current_tab == AppTab::Extensions, "🧩 Extensions").clicked() {
                self.current_tab = AppTab::Extensions;
            }
            if ui.selectable_label(self.current_tab == AppTab::Settings, "⚙️ Settings").clicked() {
                self.current_tab = AppTab::Settings;
            }
        });

        // Main content area
        egui::CentralPanel::default().show(ctx, |ui| {
            match self.current_tab {
                AppTab::Dashboard => {
                    ui.heading("Dashboard");
                    ui.label("Welcome to NovaPcSuite!");
                    ui.separator();
                    
                    // Show some basic stats
                    ui.horizontal(|ui| {
                        ui.label("Active Plugins:");
                        ui.label("Loading...");
                    });
                }
                AppTab::Backup => {
                    ui.heading("Backup Management");
                    ui.label("Backup functionality will be implemented here.");
                }
                AppTab::Extensions => {
                    self.extensions_ui.update(ui, ctx);
                }
                AppTab::Settings => {
                    ui.heading("Settings");
                    ui.label("Application settings will be implemented here.");
                }
            }
        });
    }
}