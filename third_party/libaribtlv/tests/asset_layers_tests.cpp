#include <aribtlv/asset_layers.hpp>

#include <cstdlib>
#include <iostream>
#include <string>

namespace {

void check(const bool condition, const std::string& message) {
    if (condition) return;
    std::cerr << "FAIL: " << message << '\n';
    std::exit(1);
}

aribtlv::TrackInfo video(const std::uint32_t context,
                         std::initializer_list<aribtlv::AssetGroupInfo> groups) {
    aribtlv::TrackInfo track;
    track.context_id = context;
    track.kind = aribtlv::TrackKind::Video;
    track.asset_groups = groups;
    return track;
}

} // namespace

int main() {
    const auto implicit = video(1, {});
    const auto preferred = video(1, {{0x10, 0}});
    const auto fallback = video(1, {{0x10, 1}});
    const auto unrelated = video(1, {{0x11, 1}});
    const auto other_context = video(2, {{0x10, 1}});

    check(aribtlv::assetSelectionLevel(implicit) == 0,
          "ungrouped video is not the implicit base layer");
    check(aribtlv::assetSelectionLevel(fallback) == 1,
          "minimum selection level was not returned");
    check(aribtlv::assetSelectionLevel(fallback, 0x10) == 1,
          "group-specific selection level was not returned");
    check(!aribtlv::assetSelectionLevel(fallback, 0x11).has_value(),
          "missing group returned a selection level");
    check(aribtlv::sharesVideoAssetGroup(preferred, fallback),
          "explicitly grouped layers did not match");
    check(aribtlv::sharesVideoAssetGroup(implicit, fallback),
          "implicit base layer did not match grouped enhancement layer");
    check(!aribtlv::sharesVideoAssetGroup(preferred, unrelated),
          "unrelated groups matched");
    check(!aribtlv::sharesVideoAssetGroup(preferred, other_context),
          "tracks from different contexts matched");

    std::cout << "asset layer tests passed\n";
}
