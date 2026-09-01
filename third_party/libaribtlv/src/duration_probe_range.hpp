#pragma once

#include <algorithm>
#include <cstdint>
#include <optional>
#include <vector>

namespace aribtlv::detail {

struct VideoPresentationBoundary {
    std::optional<std::int64_t> start_us;
    std::optional<std::int64_t> end_us;
};

struct VideoPresentationRange {
    std::int64_t start_us = 0;
    std::int64_t end_us = 0;
};

inline std::optional<VideoPresentationRange> union_video_presentation_ranges(
    const std::vector<VideoPresentationBoundary>& tracks) noexcept {
    std::optional<std::int64_t> start;
    std::optional<std::int64_t> end;
    for (const auto& track : tracks) {
        if (track.start_us.has_value() &&
            (!start.has_value() || *track.start_us < *start)) {
            start = track.start_us;
        }
        if (track.end_us.has_value() &&
            (!end.has_value() || *track.end_us > *end)) {
            end = track.end_us;
        }
    }
    if (!start.has_value() || !end.has_value() || *end < *start) return std::nullopt;
    return VideoPresentationRange{*start, *end};
}

} // namespace aribtlv::detail
