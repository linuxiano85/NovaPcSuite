//! Platform event system for plugin communication.
//! 
//! This module defines events that can be consumed by plugins and provides
//! an event bus for distributing events throughout the system.

use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::sync::{Arc, Mutex};
use tokio::sync::broadcast;
use uuid::Uuid;

/// Platform events that plugins can subscribe to
#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum PlatformEvent {
    /// Backup operation started
    BackupStarted {
        backup_id: Uuid,
        source_path: std::path::PathBuf,
        label: String,
    },
    /// Backup operation completed
    BackupCompleted {
        backup_id: Uuid,
        manifest_id: Uuid,
        files_processed: usize,
        total_size: u64,
        duration_ms: u64,
    },
    /// Backup operation failed
    BackupFailed {
        backup_id: Uuid,
        error: String,
    },
    /// File being processed
    FileProcessing {
        backup_id: Uuid,
        file_path: std::path::PathBuf,
        progress: f64, // 0.0 to 1.0
    },
    /// Chunk created
    ChunkCreated {
        backup_id: Uuid,
        chunk_id: String,
        size: u64,
        is_duplicate: bool,
    },
    /// Restore operation started
    RestoreStarted {
        restore_id: Uuid,
        manifest_id: Uuid,
        target_path: std::path::PathBuf,
    },
    /// Restore operation completed
    RestoreCompleted {
        restore_id: Uuid,
        files_restored: usize,
        total_size: u64,
        duration_ms: u64,
    },
    /// System health check
    SystemHealth {
        cpu_usage: f64,
        memory_usage: f64,
        disk_usage: f64,
        timestamp: chrono::DateTime<chrono::Utc>,
    },
    /// Device connected/disconnected
    DeviceEvent {
        device_id: String,
        device_type: DeviceType,
        event_type: DeviceEventType,
    },
    /// Telephony event (calls, SMS, notifications)
    TelephonyEvent {
        event_type: TelephonyEventType,
        data: HashMap<String, String>,
    },
}

/// Type of device
#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum DeviceType {
    Mobile,
    Tablet,
    Desktop,
    Server,
    Unknown,
}

/// Device event type
#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum DeviceEventType {
    Connected,
    Disconnected,
    StatusChanged,
}

/// Telephony event type
#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum TelephonyEventType {
    IncomingCall,
    OutgoingCall,
    CallEnded,
    SmsReceived,
    SmsSent,
    NotificationSent,
}

/// Event bus for distributing platform events
#[derive(Debug)]
pub struct EventBus {
    sender: broadcast::Sender<PlatformEvent>,
    subscribers: Arc<Mutex<HashMap<String, broadcast::Receiver<PlatformEvent>>>>,
}

impl EventBus {
    /// Create a new event bus
    pub fn new() -> Self {
        let (sender, _) = broadcast::channel(1000);
        
        Self {
            sender,
            subscribers: Arc::new(Mutex::new(HashMap::new())),
        }
    }

    /// Publish an event to all subscribers
    pub fn publish(&self, event: PlatformEvent) -> Result<usize, broadcast::error::SendError<PlatformEvent>> {
        self.sender.send(event)
    }

    /// Subscribe to events with a unique subscription ID
    pub fn subscribe(&self, subscription_id: String) -> broadcast::Receiver<PlatformEvent> {
        let receiver = self.sender.subscribe();
        self.subscribers.lock().unwrap().insert(subscription_id, receiver.resubscribe());
        receiver
    }

    /// Unsubscribe from events
    pub fn unsubscribe(&self, subscription_id: &str) {
        self.subscribers.lock().unwrap().remove(subscription_id);
    }

    /// Get the number of active subscribers
    pub fn subscriber_count(&self) -> usize {
        self.sender.receiver_count()
    }

    /// Create a scoped event publisher for a specific operation
    pub fn create_scoped_publisher(&self, operation_id: Uuid) -> ScopedEventPublisher<'_> {
        ScopedEventPublisher {
            bus: self,
            operation_id,
        }
    }
}

impl Default for EventBus {
    fn default() -> Self {
        Self::new()
    }
}

/// Scoped event publisher that automatically includes operation context
#[derive(Debug)]
pub struct ScopedEventPublisher<'a> {
    bus: &'a EventBus,
    operation_id: Uuid,
}

impl<'a> ScopedEventPublisher<'a> {
    /// Publish a backup-related event
    pub fn backup_started(&self, source_path: std::path::PathBuf, label: String) -> Result<usize, broadcast::error::SendError<PlatformEvent>> {
        self.bus.publish(PlatformEvent::BackupStarted {
            backup_id: self.operation_id,
            source_path,
            label,
        })
    }

    /// Publish backup completion event
    pub fn backup_completed(&self, manifest_id: Uuid, files_processed: usize, total_size: u64, duration_ms: u64) -> Result<usize, broadcast::error::SendError<PlatformEvent>> {
        self.bus.publish(PlatformEvent::BackupCompleted {
            backup_id: self.operation_id,
            manifest_id,
            files_processed,
            total_size,
            duration_ms,
        })
    }

    /// Publish backup failure event
    pub fn backup_failed(&self, error: String) -> Result<usize, broadcast::error::SendError<PlatformEvent>> {
        self.bus.publish(PlatformEvent::BackupFailed {
            backup_id: self.operation_id,
            error,
        })
    }

    /// Publish file processing event
    pub fn file_processing(&self, file_path: std::path::PathBuf, progress: f64) -> Result<usize, broadcast::error::SendError<PlatformEvent>> {
        self.bus.publish(PlatformEvent::FileProcessing {
            backup_id: self.operation_id,
            file_path,
            progress,
        })
    }

    /// Publish chunk creation event
    pub fn chunk_created(&self, chunk_id: String, size: u64, is_duplicate: bool) -> Result<usize, broadcast::error::SendError<PlatformEvent>> {
        self.bus.publish(PlatformEvent::ChunkCreated {
            backup_id: self.operation_id,
            chunk_id,
            size,
            is_duplicate,
        })
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use tokio::time::{sleep, Duration};

    #[tokio::test]
    async fn test_event_bus_basic() {
        let bus = EventBus::new();
        let mut receiver = bus.subscribe("test-subscriber".to_string());

        let backup_id = Uuid::new_v4();
        let event = PlatformEvent::BackupStarted {
            backup_id,
            source_path: "/test/path".into(),
            label: "test-backup".to_string(),
        };

        let send_result = bus.publish(event.clone());
        assert!(send_result.is_ok());

        // Small delay to ensure event is processed
        sleep(Duration::from_millis(10)).await;

        let received_event = receiver.try_recv();
        assert!(received_event.is_ok());
        
        if let Ok(received) = received_event {
            match received {
                PlatformEvent::BackupStarted { backup_id: received_id, .. } => {
                    assert_eq!(received_id, backup_id);
                }
                _ => panic!("Unexpected event type received"),
            }
        }
    }

    #[tokio::test]
    async fn test_scoped_publisher() {
        let bus = EventBus::new();
        let mut receiver = bus.subscribe("scoped-test".to_string());

        let operation_id = Uuid::new_v4();
        let publisher = bus.create_scoped_publisher(operation_id);

        let result = publisher.backup_started("/test".into(), "test".to_string());
        assert!(result.is_ok());

        sleep(Duration::from_millis(10)).await;

        let received_event = receiver.try_recv();
        assert!(received_event.is_ok());
    }

    #[test]
    fn test_event_serialization() {
        let event = PlatformEvent::BackupStarted {
            backup_id: Uuid::new_v4(),
            source_path: "/test/path".into(),
            label: "test-backup".to_string(),
        };

        let serialized = serde_json::to_string(&event).unwrap();
        let deserialized: PlatformEvent = serde_json::from_str(&serialized).unwrap();

        match (event, deserialized) {
            (PlatformEvent::BackupStarted { backup_id: id1, .. }, PlatformEvent::BackupStarted { backup_id: id2, .. }) => {
                assert_eq!(id1, id2);
            }
            _ => panic!("Event serialization/deserialization failed"),
        }
    }
}