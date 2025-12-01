package jetstream

/*
#cgo LDFLAGS: -L${SRCDIR}/../target/release -ljsp_c
#include "../jsp_c/jetstream_proto.h"
#include <stdlib.h>
*/
import "C"
import (
	"errors"
	"unsafe"
)

// DeliveryMode represents the delivery guarantee mode
type DeliveryMode int

const (
	// Reliable guarantees delivery with retransmission
	Reliable DeliveryMode = 0
	// BestEffort provides no delivery guarantees
	BestEffort DeliveryMode = 1
	// PartiallyReliable provides time-limited retries
	PartiallyReliable DeliveryMode = 2
)

// Connection represents a JetStream connection
type Connection struct {
	handle *C.JspConnection
}

// NewConnection creates a new JetStream connection
func NewConnection() (*Connection, error) {
	handle := C.jsp_connection_new()
	if handle == nil {
		return nil, errors.New("failed to create connection")
	}
	return &Connection{handle: handle}, nil
}

// Connect connects to a JetStream server
func (c *Connection) Connect(addr string) error {
	cAddr := C.CString(addr)
	defer C.free(unsafe.Pointer(cAddr))

	err := C.jsp_connection_connect(c.handle, cAddr)
	if err != C.JSP_ERROR_SUCCESS {
		return errors.New(C.GoString(C.jsp_error_message(err)))
	}
	return nil
}

// Handshake performs the handshake with the server
func (c *Connection) Handshake() error {
	err := C.jsp_connection_handshake(c.handle)
	if err != C.JSP_ERROR_SUCCESS {
		return errors.New(C.GoString(C.jsp_error_message(err)))
	}
	return nil
}

// SessionID returns the session ID
func (c *Connection) SessionID() uint64 {
	return uint64(C.jsp_connection_session_id(c.handle))
}

// OpenStream opens a new stream with the specified priority and delivery mode
func (c *Connection) OpenStream(priority uint8, mode DeliveryMode) (uint32, error) {
	var streamID C.uint
	err := C.jsp_connection_open_stream(
		c.handle,
		C.uint(priority),
		C.JspDeliveryMode(mode),
		&streamID,
	)
	if err != C.JSP_ERROR_SUCCESS {
		return 0, errors.New(C.GoString(C.jsp_error_message(err)))
	}
	return uint32(streamID), nil
}

// Send sends data on the specified stream
func (c *Connection) Send(streamID uint32, data []byte) error {
	if len(data) == 0 {
		return nil
	}

	err := C.jsp_connection_send(
		c.handle,
		C.uint(streamID),
		(*C.uint8_t)(unsafe.Pointer(&data[0])),
		C.size_t(len(data)),
	)
	if err != C.JSP_ERROR_SUCCESS {
		return errors.New(C.GoString(C.jsp_error_message(err)))
	}
	return nil
}

// Close closes the connection gracefully
func (c *Connection) Close() error {
	err := C.jsp_connection_close(c.handle)
	if err != C.JSP_ERROR_SUCCESS {
		return errors.New(C.GoString(C.jsp_error_message(err)))
	}
	return nil
}

// Free releases the connection resources
func (c *Connection) Free() {
	if c.handle != nil {
		C.jsp_connection_free(c.handle)
		c.handle = nil
	}
}
