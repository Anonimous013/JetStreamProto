#pragma once
#include "rust/cxx.h"
#include <memory>
#include <string>
#include <vector>

namespace jetstream {

struct ConnectionConfig {
    std::string addr;
    uint64_t timeout_ms;
};

class Connection {
public:
    Connection(const std::string& addr);
    ~Connection();

    void connect();
    void send(uint32_t stream_id, const std::vector<uint8_t>& data);
    std::vector<uint8_t> receive(uint32_t& stream_id);
    void close();

private:
    class impl;
    std::unique_ptr<impl> impl;
};

std::unique_ptr<Connection> new_connection(const std::string& addr);

} // namespace jetstream
