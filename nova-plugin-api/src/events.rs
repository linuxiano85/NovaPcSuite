use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::sync::Arc;
use tokio::sync::{broadcast, RwLock};
use uuid::Uuid;

/// Event bus for plugin communication
#[derive(Debug)]
pub struct EventBus {
    sender: broadcast::Sender<NovaEvent>,
    subscribers: Arc<RwLock<HashMap<String, PluginEventSubscription>>>,
}

impl EventBus {
    pub fn new() -> Self {
        let (sender, _) = broadcast::channel(1000);
        Self {
            sender,
            subscribers: Arc::new(RwLock::new(HashMap::new())),
        }
    }

    /// Publish an event to all subscribers
    pub async fn publish(&self, event: NovaEvent) -> anyhow::Result<()> {
        match self.sender.send(event) {
            Ok(subscriber_count) => {
                tracing::debug!("Published event to {} subscribers", subscriber_count);
                Ok(())
            }
            Err(_) => {
                tracing::warn!("No subscribers for event");
                Ok(())
            }
        }
    }

    /// Subscribe to events with a filter
    pub async fn subscribe(&self, plugin_id: String, filter: EventFilter) -> EventSubscription {
        let receiver = self.sender.subscribe();
        let subscription_id = Uuid::new_v4().to_string();
        
        let subscription = PluginEventSubscription {
            plugin_id: plugin_id.clone(),
            filter,
            subscription_id: subscription_id.clone(),
        };

        let mut subscribers = self.subscribers.write().await;
        subscribers.insert(subscription_id.clone(), subscription);

        EventSubscription {
            id: subscription_id,
            receiver,
        }
    }

    /// Unsubscribe from events
    pub async fn unsubscribe(&self, subscription_id: &str) {
        let mut subscribers = self.subscribers.write().await;
        subscribers.remove(subscription_id);
    }

    /// Get subscriber count
    pub fn subscriber_count(&self) -> usize {
        self.sender.receiver_count()
    }
}

/// Event subscription handle
pub struct EventSubscription {
    pub id: String,
    pub receiver: broadcast::Receiver<NovaEvent>,
}

/// Plugin event subscription info
#[derive(Debug, Clone)]
pub struct PluginEventSubscription {
    pub plugin_id: String,
    pub filter: EventFilter,
    pub subscription_id: String,
}

/// Filter for events that a plugin wants to receive
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct EventFilter {
    pub event_types: Vec<EventType>,
    pub include_system: bool,
    pub include_user: bool,
}

impl Default for EventFilter {
    fn default() -> Self {
        Self {
            event_types: vec![EventType::All],
            include_system: true,
            include_user: true,
        }
    }
}

/// Types of events in the system
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub enum EventType {
    All,
    BackupStarted,
    BackupCompleted,
    BackupFailed,
    FileChanged,
    SystemInfo,
    ProximityChanged,
    TelephonyEvent,
    PluginLoaded,
    PluginUnloaded,
    ConfigChanged,
}

/// Events that can be published in the system
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct NovaEvent {
    pub id: String,
    pub timestamp: chrono::DateTime<chrono::Utc>,
    pub event_type: EventType,
    pub source: String,
    pub data: serde_json::Value,
}

impl NovaEvent {
    pub fn new(event_type: EventType, source: String, data: serde_json::Value) -> Self {
        Self {
            id: Uuid::new_v4().to_string(),
            timestamp: chrono::Utc::now(),
            event_type,
            source,
            data,
        }
    }

    /// Create a backup started event
    pub fn backup_started(source: String, backup_id: String) -> Self {
        Self::new(
            EventType::BackupStarted,
            source,
            serde_json::json!({ "backup_id": backup_id }),
        )
    }

    /// Create a backup completed event
    pub fn backup_completed(source: String, backup_id: String, files_count: usize) -> Self {
        Self::new(
            EventType::BackupCompleted,
            source,
            serde_json::json!({
                "backup_id": backup_id,
                "files_count": files_count
            }),
        )
    }

    /// Create a plugin loaded event
    pub fn plugin_loaded(plugin_id: String) -> Self {
        Self::new(
            EventType::PluginLoaded,
            "system".to_string(),
            serde_json::json!({ "plugin_id": plugin_id }),
        )
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[tokio::test]
    async fn test_event_bus_publish_subscribe() {
        let event_bus = EventBus::new();
        
        // Subscribe to events
        let mut subscription = event_bus
            .subscribe("test-plugin".to_string(), EventFilter::default())
            .await;

        // Publish an event
        let event = NovaEvent::backup_started("test".to_string(), "backup123".to_string());
        event_bus.publish(event.clone()).await.unwrap();

        // Receive the event
        let received_event = subscription.receiver.recv().await.unwrap();
        assert_eq!(received_event.event_type, EventType::BackupStarted);
        assert_eq!(received_event.source, "test");
    }

    #[tokio::test]
    async fn test_multiple_subscribers() {
        let event_bus = EventBus::new();
        
        let _sub1 = event_bus
            .subscribe("plugin1".to_string(), EventFilter::default())
            .await;
        let _sub2 = event_bus
            .subscribe("plugin2".to_string(), EventFilter::default())
            .await;

        assert_eq!(event_bus.subscriber_count(), 2);
    }
}