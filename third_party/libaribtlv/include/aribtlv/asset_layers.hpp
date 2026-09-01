#pragma once

#include <aribtlv/types.hpp>

#include <cstdint>
#include <optional>

namespace aribtlv {

// Returns the selection level for one asset group. An ungrouped video is the
// implicit base layer (level 0); other ungrouped tracks have no layer level.
std::optional<std::uint8_t> assetSelectionLevel(
    const TrackInfo& track,
    std::optional<std::uint8_t> group_identification = std::nullopt) noexcept;

bool belongsToAssetGroup(const TrackInfo& track,
                         std::uint8_t group_identification) noexcept;

// Ungrouped video and a grouped video in the same package context form an
// implicit base/enhancement relationship defined by ARIB asset grouping.
bool sharesVideoAssetGroup(const TrackInfo& left, const TrackInfo& right) noexcept;

} // namespace aribtlv
