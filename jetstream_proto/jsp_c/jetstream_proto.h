#ifndef JETSTREAM_PROTO_H
#define JETSTREAM_PROTO_H

#pragma once

#include <stdarg.h>
#include <stdbool.h>
#include <stdint.h>
#include <stdlib.h>

/**
 * Delivery modes
 */
typedef enum JspDeliveryMode {
  Reliable = 0,
  BestEffort = 1,
  PartiallyReliable = 2,
} JspDeliveryMode;

/**
 * Error codes
 */
typedef enum JspError {
  Success = 0,
  NullPointer = 1,
  ConnectionFailed = 2,
  HandshakeFailed = 3,
  SendFailed = 4,
  ReceiveFailed = 5,
  InvalidMode = 6,
  NotConnected = 7,
} JspError;

/**
 * Opaque connection handle
 */
typedef struct JspConnection JspConnection;

/**
 * Create a new connection
 * Returns NULL on failure
 */
struct JspConnection *jsp_connection_new(void);

/**
 * Connect to a server
 * @param conn - Connection handle
 * @param addr - Server address (null-terminated string)
 * @return Error code
 */
enum JspError jsp_connection_connect(struct JspConnection *conn, const char *addr);

/**
 * Perform handshake
 * @param conn - Connection handle
 * @return Error code
 */
enum JspError jsp_connection_handshake(struct JspConnection *conn);

/**
 * Get session ID
 * @param conn - Connection handle
 * @return Session ID (0 if not connected)
 */
unsigned long long jsp_connection_session_id(const struct JspConnection *conn);

/**
 * Open a new stream
 * @param conn - Connection handle
 * @param priority - Stream priority (0-255)
 * @param mode - Delivery mode
 * @param stream_id_out - Output parameter for stream ID
 * @return Error code
 */
enum JspError jsp_connection_open_stream(struct JspConnection *conn,
                                         unsigned int priority,
                                         enum JspDeliveryMode mode,
                                         unsigned int *stream_id_out);

/**
 * Send data on a stream
 * @param conn - Connection handle
 * @param stream_id - Stream ID
 * @param data - Data buffer
 * @param len - Data length
 * @return Error code
 */
enum JspError jsp_connection_send(struct JspConnection *conn,
                                  unsigned int stream_id,
                                  const uint8_t *data,
                                  uintptr_t len);

/**
 * Close connection
 * @param conn - Connection handle
 * @return Error code
 */
enum JspError jsp_connection_close(struct JspConnection *conn);

/**
 * Free connection
 * @param conn - Connection handle
 */
void jsp_connection_free(struct JspConnection *conn);

/**
 * Get error message for error code
 * @param error - Error code
 * @return Error message (static string)
 */
const char *jsp_error_message(enum JspError error);

#endif /* JETSTREAM_PROTO_H */
