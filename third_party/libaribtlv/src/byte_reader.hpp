#pragma once

#include <cstddef>
#include <cstdint>
#include <vector>

#include "parser_common.hpp"

namespace aribtlv::detail {

class ByteReader {
public:
    ByteReader(const std::uint8_t* data, const std::size_t size) noexcept
        : data_(data), size_(size) {}

    std::size_t remaining() const noexcept { return size_ - position_; }
    std::size_t position() const noexcept { return position_; }
    const std::uint8_t* current() const noexcept { return data_ + position_; }

    bool peek_u8(std::uint8_t& value) const noexcept {
        if (remaining() < 1) return false;
        value = data_[position_];
        return true;
    }

    bool read_u8(std::uint8_t& value) noexcept {
        if (!peek_u8(value)) return false;
        ++position_;
        return true;
    }

    bool read_u16(std::uint16_t& value) noexcept {
        if (remaining() < 2) return false;
        value = read_be16(current());
        position_ += 2;
        return true;
    }

    bool read_u32(std::uint32_t& value) noexcept {
        if (remaining() < 4) return false;
        value = read_be32(current());
        position_ += 4;
        return true;
    }

    bool skip(const std::size_t size) noexcept {
        if (size > remaining()) return false;
        position_ += size;
        return true;
    }

    bool read_bytes(const std::size_t size, std::vector<std::uint8_t>& value) {
        if (size > remaining()) return false;
        value.assign(current(), current() + size);
        position_ += size;
        return true;
    }

    bool read_view(const std::size_t size, const std::uint8_t*& data) noexcept {
        if (size > remaining()) return false;
        data = current();
        position_ += size;
        return true;
    }

private:
    const std::uint8_t* data_ = nullptr;
    std::size_t size_ = 0;
    std::size_t position_ = 0;
};

} // namespace aribtlv::detail

