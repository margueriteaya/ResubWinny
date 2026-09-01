#include <aribtlv/asset_layers.hpp>

#include <algorithm>

namespace aribtlv {

std::optional<std::uint8_t> assetSelectionLevel(
    const TrackInfo& track,
    const std::optional<std::uint8_t> group_identification) noexcept {
    if (track.asset_groups.empty()) {
        return track.kind == TrackKind::Video && !group_identification.has_value()
            ? std::optional<std::uint8_t>{0}
            : std::nullopt;
    }
    const auto group = std::min_element(
        track.asset_groups.begin(), track.asset_groups.end(),
        [group_identification](const AssetGroupInfo& left,
                               const AssetGroupInfo& right) {
            const bool left_matches = !group_identification.has_value() ||
                left.group_identification == *group_identification;
            const bool right_matches = !group_identification.has_value() ||
                right.group_identification == *group_identification;
            if (left_matches != right_matches) return left_matches;
            return left.selection_level < right.selection_level;
        });
    if (group == track.asset_groups.end() ||
        (group_identification.has_value() &&
         group->group_identification != *group_identification)) {
        return std::nullopt;
    }
    return group->selection_level;
}

bool belongsToAssetGroup(const TrackInfo& track,
                         const std::uint8_t group_identification) noexcept {
    return std::any_of(
        track.asset_groups.begin(), track.asset_groups.end(),
        [group_identification](const AssetGroupInfo& group) {
            return group.group_identification == group_identification;
        });
}

bool sharesVideoAssetGroup(const TrackInfo& left, const TrackInfo& right) noexcept {
    if (left.kind != TrackKind::Video || right.kind != TrackKind::Video ||
        left.context_id != right.context_id) return false;
    if (left.asset_groups.empty() && right.asset_groups.empty()) return false;
    if (left.asset_groups.empty() || right.asset_groups.empty()) return true;
    return std::any_of(
        left.asset_groups.begin(), left.asset_groups.end(),
        [&right](const AssetGroupInfo& left_group) {
            return belongsToAssetGroup(right, left_group.group_identification);
        });
}

} // namespace aribtlv
