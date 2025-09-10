package progress

import (
	"fmt"
	"sync"
	"time"
)

// EventType represents different types of progress events
type EventType string

const (
	EventScanStart     EventType = "scan_start"
	EventScanProgress  EventType = "scan_progress"
	EventScanComplete  EventType = "scan_complete"
	EventPlanStart     EventType = "plan_start"
	EventPlanProgress  EventType = "plan_progress"
	EventPlanComplete  EventType = "plan_complete"
	EventBackupStart   EventType = "backup_start"
	EventBackupProgress EventType = "backup_progress"
	EventBackupComplete EventType = "backup_complete"
	EventError         EventType = "error"
	EventInfo          EventType = "info"
)

// Event represents a progress event
type Event struct {
	Type        EventType              `json:"type"`
	Timestamp   time.Time              `json:"timestamp"`
	Message     string                 `json:"message"`
	Progress    float64                `json:"progress"` // 0.0 to 1.0
	Total       int64                  `json:"total"`
	Current     int64                  `json:"current"`
	Speed       int64                  `json:"speed"` // bytes per second
	ETA         time.Duration          `json:"eta"`
	Metadata    map[string]interface{} `json:"metadata,omitempty"`
}

// Handler is a function that handles progress events
type Handler func(event *Event)

// Broadcaster manages progress event broadcasting
type Broadcaster struct {
	handlers []Handler
	mu       sync.RWMutex
}

// NewBroadcaster creates a new progress broadcaster
func NewBroadcaster() *Broadcaster {
	return &Broadcaster{
		handlers: make([]Handler, 0),
	}
}

// AddHandler adds a progress event handler
func (b *Broadcaster) AddHandler(handler Handler) {
	b.mu.Lock()
	defer b.mu.Unlock()
	b.handlers = append(b.handlers, handler)
}

// Broadcast sends an event to all registered handlers
func (b *Broadcaster) Broadcast(event *Event) {
	b.mu.RLock()
	defer b.mu.RUnlock()
	
	for _, handler := range b.handlers {
		go handler(event) // Run handlers concurrently
	}
}

// EmitEvent creates and broadcasts an event
func (b *Broadcaster) EmitEvent(eventType EventType, message string, progress float64, current, total int64) {
	event := &Event{
		Type:      eventType,
		Timestamp: time.Now(),
		Message:   message,
		Progress:  progress,
		Current:   current,
		Total:     total,
		Metadata:  make(map[string]interface{}),
	}
	
	// Calculate ETA if we have progress info
	if progress > 0 && progress < 1.0 {
		elapsed := time.Since(event.Timestamp)
		estimated := time.Duration(float64(elapsed) / progress)
		event.ETA = estimated - elapsed
	}
	
	b.Broadcast(event)
}

// EmitError emits an error event
func (b *Broadcaster) EmitError(err error) {
	event := &Event{
		Type:      EventError,
		Timestamp: time.Now(),
		Message:   err.Error(),
		Metadata:  make(map[string]interface{}),
	}
	b.Broadcast(event)
}

// EmitInfo emits an info event
func (b *Broadcaster) EmitInfo(message string) {
	event := &Event{
		Type:      EventInfo,
		Timestamp: time.Now(),
		Message:   message,
		Metadata:  make(map[string]interface{}),
	}
	b.Broadcast(event)
}

// Tracker helps track progress for long-running operations
type Tracker struct {
	broadcaster *Broadcaster
	eventType   EventType
	total       int64
	current     int64
	startTime   time.Time
	lastUpdate  time.Time
	speed       int64
}

// NewTracker creates a new progress tracker
func NewTracker(broadcaster *Broadcaster, eventType EventType, total int64) *Tracker {
	return &Tracker{
		broadcaster: broadcaster,
		eventType:   eventType,
		total:      total,
		startTime:  time.Now(),
		lastUpdate: time.Now(),
	}
}

// Update updates the progress
func (t *Tracker) Update(current int64, message string) {
	t.current = current
	
	// Calculate speed
	now := time.Now()
	if now.Sub(t.lastUpdate) > time.Second {
		elapsed := now.Sub(t.startTime).Seconds()
		if elapsed > 0 {
			t.speed = int64(float64(t.current) / elapsed)
		}
		t.lastUpdate = now
	}
	
	// Calculate progress
	progress := float64(t.current) / float64(t.total)
	if t.total == 0 {
		progress = 0
	}
	
	// Emit event
	event := &Event{
		Type:      t.eventType,
		Timestamp: now,
		Message:   message,
		Progress:  progress,
		Current:   t.current,
		Total:     t.total,
		Speed:     t.speed,
		Metadata:  make(map[string]interface{}),
	}
	
	// Calculate ETA
	if progress > 0 && progress < 1.0 && t.speed > 0 {
		remaining := t.total - t.current
		event.ETA = time.Duration(float64(remaining)/float64(t.speed)) * time.Second
	}
	
	t.broadcaster.Broadcast(event)
}

// Complete marks the operation as complete
func (t *Tracker) Complete(message string) {
	event := &Event{
		Type:      t.eventType,
		Timestamp: time.Now(),
		Message:   message,
		Progress:  1.0,
		Current:   t.total,
		Total:     t.total,
		Speed:     t.speed,
		Metadata:  make(map[string]interface{}),
	}
	t.broadcaster.Broadcast(event)
}

// ConsoleHandler prints events to console
func ConsoleHandler(event *Event) {
	switch event.Type {
	case EventError:
		fmt.Printf("ERROR: %s\n", event.Message)
	case EventInfo:
		fmt.Printf("INFO: %s\n", event.Message)
	default:
		if event.Total > 0 {
			fmt.Printf("[%s] %s - %d/%d (%.1f%%) Speed: %d B/s\n",
				event.Type, event.Message, event.Current, event.Total,
				event.Progress*100, event.Speed)
		} else {
			fmt.Printf("[%s] %s\n", event.Type, event.Message)
		}
	}
}