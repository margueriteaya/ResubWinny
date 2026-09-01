#pragma once

#include <cstddef>
#include <cstdint>
#include <functional>
#include <vector>

#include "parser_common.hpp"

namespace aribtlv::detail {

struct TlvPacketView {
    std::uint8_t type = 0;
    const std::uint8_t* payload = nullptr;
    std::size_t size = 0;
    std::uint64_t input_offset = 0;
};

class TlvParser {
public:
    using PacketCallback = std::function<void(const TlvPacketView&)>;
    using ResyncCallback = std::function<void(std::uint64_t)>;

    TlvParser(const Limits&, PacketCallback, ResyncCallback, ErrorCallback);

    void push(const std::uint8_t* data, std::size_t size);
    void flush();
    void reset(std::uint64_t input_offset = 0);

private:
    bool candidate_type(std::uint8_t type) const noexcept;
    bool find_boundary(bool end_of_stream);
    void process(bool end_of_stream);
    void consume(std::size_t size);
    void compact();
    std::size_t available() const noexcept;
    std::uint64_t current_offset() const noexcept;

    Limits limits_;
    PacketCallback on_packet_;
    ResyncCallback on_resync_;
    ErrorCallback on_error_;
    std::vector<std::uint8_t> buffer_;
    std::size_t cursor_ = 0;
    std::uint64_t buffer_offset_ = 0;
    bool synchronized_ = false;
};

} // namespace aribtlv::detail
