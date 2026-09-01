#include "tlv_parser.hpp"

#include <algorithm>
#include <limits>
#include <utility>

namespace aribtlv::detail {

TlvParser::TlvParser(const Limits& limits, PacketCallback on_packet,
                     ResyncCallback on_resync, ErrorCallback on_error)
    : limits_(limits), on_packet_(std::move(on_packet)),
      on_resync_(std::move(on_resync)), on_error_(std::move(on_error)) {}

void TlvParser::push(const std::uint8_t* data, const std::size_t size) {
    if (size == 0) return;

    std::size_t input_cursor = 0;
    if (!synchronized_) {
        buffer_.insert(buffer_.end(), data, data + size);
        process(false);
        return;
    }

    // Complete only the packet that crossed the previous push boundary. Once
    // it is emitted, the remaining complete packets can borrow the caller's
    // input for the duration of the synchronous callback.
    while (input_cursor < size && !buffer_.empty() && synchronized_) {
        std::size_t required = 0;
        if (buffer_.size() < 4) {
            required = 4 - buffer_.size();
        } else {
            if (buffer_[0] != 0x7f || !candidate_type(buffer_[1])) {
                buffer_.insert(buffer_.end(), data + input_cursor, data + size);
                process(false);
                return;
            }
            const auto payload_size = static_cast<std::size_t>(read_be16(buffer_.data() + 2));
            if (payload_size > limits_.max_tlv_payload) {
                buffer_.insert(buffer_.end(), data + input_cursor, data + size);
                process(false);
                return;
            }
            const auto packet_size = 4 + payload_size;
            required = packet_size > buffer_.size() ? packet_size - buffer_.size() : 0;
        }

        const auto copied = std::min(required, size - input_cursor);
        buffer_.insert(buffer_.end(), data + input_cursor, data + input_cursor + copied);
        input_cursor += copied;
        if (copied < required) return;
        process(false);
    }

    if (input_cursor == size) return;
    if (!synchronized_ || !buffer_.empty()) {
        buffer_.insert(buffer_.end(), data + input_cursor, data + size);
        process(false);
        return;
    }

    const auto direct_begin = input_cursor;
    while (size - input_cursor >= 4) {
        const auto* header = data + input_cursor;
        if (header[0] != 0x7f || !candidate_type(header[1])) break;
        const auto payload_size = static_cast<std::size_t>(read_be16(header + 2));
        if (payload_size > limits_.max_tlv_payload) break;
        const auto packet_size = 4 + payload_size;
        if (packet_size > size - input_cursor) break;
        on_packet_(TlvPacketView{
            header[1], header + 4, payload_size,
            buffer_offset_ + static_cast<std::uint64_t>(input_cursor - direct_begin),
        });
        input_cursor += packet_size;
    }

    buffer_offset_ += static_cast<std::uint64_t>(input_cursor - direct_begin);
    if (input_cursor != size) {
        buffer_.insert(buffer_.end(), data + input_cursor, data + size);
        process(false);
    }
}

void TlvParser::flush() {
    process(true);
    synchronized_ = false;
}

void TlvParser::reset(const std::uint64_t input_offset) {
    buffer_.clear();
    cursor_ = 0;
    buffer_offset_ = input_offset;
    synchronized_ = false;
}

bool TlvParser::candidate_type(const std::uint8_t type) const noexcept {
    switch (type) {
    case 0x01:
    case 0x02:
    case 0x03:
    case 0xfe:
    case 0xff:
        return true;
    default:
        return false;
    }
}

std::size_t TlvParser::available() const noexcept {
    return buffer_.size() - cursor_;
}

std::uint64_t TlvParser::current_offset() const noexcept {
    return buffer_offset_ + static_cast<std::uint64_t>(cursor_);
}

void TlvParser::compact() {
    if (cursor_ == 0) {
        return;
    }
    if (cursor_ == buffer_.size()) {
        buffer_.clear();
    } else {
        buffer_.erase(buffer_.begin(),
                      buffer_.begin() + static_cast<std::ptrdiff_t>(cursor_));
    }
    buffer_offset_ += static_cast<std::uint64_t>(cursor_);
    cursor_ = 0;
}

void TlvParser::consume(const std::size_t size) {
    cursor_ += size;
}

bool TlvParser::find_boundary(const bool end_of_stream) {
    const auto begin = cursor_;
    std::size_t index = cursor_;

    while (index + 4 <= buffer_.size()) {
        if (buffer_[index] != 0x7f || !candidate_type(buffer_[index + 1])) {
            ++index;
            continue;
        }

        const auto payload_size = static_cast<std::size_t>(read_be16(buffer_.data() + index + 2));
        if (payload_size > limits_.max_tlv_payload) {
            ++index;
            continue;
        }
        const auto next = index + 4 + payload_size;
        if (next > buffer_.size()) {
            // A corrupt stream can contain a false 0x7f candidate with a very
            // large length before a complete, valid packet later in the same
            // buffer. Keep the candidate buffered, but continue looking for a
            // boundary that can already be validated.
            ++index;
            continue;
        }

        const bool final_complete_packet = end_of_stream && next == buffer_.size();
        const bool next_header_available = next + 4 <= buffer_.size();
        const bool next_header_plausible = next_header_available &&
            buffer_[next] == 0x7f && candidate_type(buffer_[next + 1]);
        if (!final_complete_packet && !next_header_plausible) {
            ++index;
            continue;
        }

        if (index != begin) {
            on_error_(ErrorCode::MalformedInput, current_offset(), true,
                      "discarded bytes while searching for a validated TLV boundary");
            consume(index - begin);
        }
        synchronized_ = true;
        return true;
    }

    if (end_of_stream) {
        if (available() != 0) {
            on_error_(ErrorCode::MalformedInput, current_offset(), true,
                      "discarded incomplete trailing TLV data at end of input");
            consume(available());
        }
        return false;
    }

    if (available() > limits_.max_resync_buffer) {
        const auto discard_size = available() - std::min<std::size_t>(3, available());
        on_error_(ErrorCode::ResourceLimit, current_offset(), true,
                  "TLV resynchronization buffer limit exceeded");
        consume(discard_size);
    }
    return false;
}

void TlvParser::process(const bool end_of_stream) {
    for (;;) {
        if (!synchronized_ && !find_boundary(end_of_stream)) {
            compact();
            return;
        }

        if (available() < 4) {
            if (end_of_stream && available() != 0) {
                on_error_(ErrorCode::MalformedInput, current_offset(), true,
                          "incomplete TLV header at end of input");
                consume(available());
            }
            compact();
            return;
        }

        const auto* header = buffer_.data() + cursor_;
        if (header[0] != 0x7f) {
            on_error_(ErrorCode::MalformedInput, current_offset(), true,
                      "lost TLV synchronization");
            on_resync_(current_offset());
            synchronized_ = false;
            consume(1);
            continue;
        }

        const auto payload_size = static_cast<std::size_t>(read_be16(header + 2));
        if (payload_size > limits_.max_tlv_payload) {
            on_error_(ErrorCode::ResourceLimit, current_offset(), true,
                      "TLV payload length exceeds configured limit");
            on_resync_(current_offset());
            synchronized_ = false;
            consume(1);
            continue;
        }
        const auto packet_size = 4 + payload_size;
        if (available() < packet_size) {
            if (end_of_stream) {
                on_error_(ErrorCode::MalformedInput, current_offset(), true,
                          "incomplete TLV payload at end of input");
                consume(available());
            }
            compact();
            return;
        }

        const TlvPacketView packet{
            header[1], header + 4, payload_size, current_offset(),
        };
        on_packet_(packet);
        consume(packet_size);
    }
}

} // namespace aribtlv::detail
