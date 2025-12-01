package jetstream

import (
	"testing"
)

func TestNewConnection(t *testing.T) {
	conn, err := NewConnection()
	if err != nil {
		t.Fatalf("Failed to create connection: %v", err)
	}
	defer conn.Free()

	if conn.handle == nil {
		t.Error("Connection handle is nil")
	}
}

func TestSessionID(t *testing.T) {
	conn, err := NewConnection()
	if err != nil {
		t.Fatalf("Failed to create connection: %v", err)
	}
	defer conn.Free()

	// Session ID should be 0 before connection
	sessionID := conn.SessionID()
	if sessionID != 0 {
		t.Errorf("Expected session ID 0, got %d", sessionID)
	}
}

func TestDeliveryModes(t *testing.T) {
	modes := []DeliveryMode{Reliable, BestEffort, PartiallyReliable}
	
	for _, mode := range modes {
		if mode < 0 || mode > 2 {
			t.Errorf("Invalid delivery mode value: %d", mode)
		}
	}
}
