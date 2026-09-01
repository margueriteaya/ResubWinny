#pragma once

#include <cstddef>
#include <cstdint>
#include <functional>
#include <string>

#include <aribtlv/types.hpp>

namespace aribtlv::detail {

using ErrorCallback = std::function<void(ErrorCode, std::uint64_t, bool, std::string)>;

inline std::uint16_t read_be16(const std::uint8_t* data) noexcept {
    return static_cast<std::uint16_t>(
        (static_cast<std::uint16_t>(data[0]) << 8U) |
        static_cast<std::uint16_t>(data[1]));
}

inline std::uint32_t read_be32(const std::uint8_t* data) noexcept {
    return (static_cast<std::uint32_t>(data[0]) << 24U) |
           (static_cast<std::uint32_t>(data[1]) << 16U) |
           (static_cast<std::uint32_t>(data[2]) << 8U) |
           static_cast<std::uint32_t>(data[3]);
}

inline std::uint64_t read_be64(const std::uint8_t* data) noexcept {
    return (static_cast<std::uint64_t>(read_be32(data)) << 32U) |
           static_cast<std::uint64_t>(read_be32(data + 4));
}

} // namespace aribtlv::detail
