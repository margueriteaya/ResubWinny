#pragma once

#include <cstdint>
#include <optional>

namespace aribtlv {

// ISO/IEC 23091-2 (CICP) colour indices used by the ARIB UHDTV signals
// handled by this library.  Keep the indices named at the boundary so
// consumers do not have to repeat the numeric table in policy code.
inline constexpr std::uint16_t kCicpBt709Primaries = 1;
inline constexpr std::uint16_t kCicpBt2020Primaries = 9;
inline constexpr std::uint16_t kCicpBt709Matrix = 1;
inline constexpr std::uint16_t kCicpBt2020NclMatrix = 9;

// ISO/IEC 23091-2 (CICP) transfer-characteristics values used by ARIB
// broadcast video. These describe the coded signal; they are not a tone-map
// decision by themselves.
enum class VideoTransferCharacteristics : std::uint8_t {
    Unknown = 0,
    Bt709 = 1,
    Bt2020_10 = 11,
    Bt2020_12 = 14,
    Smpte2084 = 16,
    AribHlg = 18,
};

constexpr std::optional<VideoTransferCharacteristics>
cicp_transfer_from_b60(const std::uint8_t value) noexcept {
    switch (value) {
    case 1: return VideoTransferCharacteristics::Bt709;
    case 2: return VideoTransferCharacteristics::Bt2020_10;
    case 3: return VideoTransferCharacteristics::Bt2020_12;
    case 4: return VideoTransferCharacteristics::Smpte2084;
    case 5: return VideoTransferCharacteristics::AribHlg;
    default: return std::nullopt;
    }
}

constexpr bool is_hdr_transfer(const VideoTransferCharacteristics value) noexcept {
    return value == VideoTransferCharacteristics::Smpte2084 ||
        value == VideoTransferCharacteristics::AribHlg;
}

constexpr bool is_hlg_transfer(const VideoTransferCharacteristics value) noexcept {
    return value == VideoTransferCharacteristics::AribHlg;
}

constexpr bool is_pq_transfer(const VideoTransferCharacteristics value) noexcept {
    return value == VideoTransferCharacteristics::Smpte2084;
}

constexpr bool is_bt2020_hlg(const std::uint16_t primaries,
                             const std::uint16_t transfer,
                             const std::uint16_t matrix,
                             const bool full_range) noexcept {
    return primaries == kCicpBt2020Primaries &&
        transfer == static_cast<std::uint16_t>(VideoTransferCharacteristics::AribHlg) &&
        matrix == kCicpBt2020NclMatrix && !full_range;
}

constexpr bool is_bt2020_pq(const std::uint16_t primaries,
                            const std::uint16_t transfer,
                            const std::uint16_t matrix,
                            const bool full_range) noexcept {
    return primaries == kCicpBt2020Primaries &&
        transfer == static_cast<std::uint16_t>(VideoTransferCharacteristics::Smpte2084) &&
        matrix == kCicpBt2020NclMatrix && !full_range;
}

} // namespace aribtlv
