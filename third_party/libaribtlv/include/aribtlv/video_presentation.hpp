#pragma once

#include <aribtlv/types.hpp>

namespace aribtlv {

// MH-EIT carries the ARIB HDR programme icon in the structured short-event
// title field. The parser records it separately from free-form text.
// Absence is deliberately not an SDR assertion: some broadcasters omit the
// icon even when the coded video is HLG.
enum class VideoPresentationHint {
    Unknown,
    Hdr,
};

VideoPresentationHint video_presentation_hint(const EventInfo& event) noexcept;

} // namespace aribtlv
