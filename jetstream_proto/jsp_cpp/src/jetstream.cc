#include "jsp_cpp/include/jetstream.h"
#include "jsp_cpp/src/lib.rs.h"
#include <stdexcept>

namespace jetstream {

class Connection::impl {
public:
    impl(const std::string& addr) : rust_conn(jsp_cpp::new_rust_connection(addr)) {}
    
    void connect() {
        rust_conn->connect();
    }

    void send(uint32_t stream_id, const std::vector<uint8_t>& data) {
        rust_conn->send(stream_id, data);
    }

    std::vector<uint8_t> receive(uint32_t& stream_id) {
        return rust_conn->receive(stream_id);
    }

    void close() {
        rust_conn->close();
    }

private:
    rust::Box<jsp_cpp::RustConnection> rust_conn;
};

Connection::Connection(const std::string& addr) : impl(new class impl(addr)) {}
Connection::~Connection() = default;

void Connection::connect() {
    impl->connect();
}

void Connection::send(uint32_t stream_id, const std::vector<uint8_t>& data) {
    impl->send(stream_id, data);
}

std::vector<uint8_t> Connection::receive(uint32_t& stream_id) {
    return impl->receive(stream_id);
}

void Connection::close() {
    impl->close();
}

std::unique_ptr<Connection> new_connection(const std::string& addr) {
    return std::make_unique<Connection>(addr);
}

} // namespace jetstream
