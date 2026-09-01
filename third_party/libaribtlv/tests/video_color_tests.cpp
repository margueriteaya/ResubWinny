#include <aribtlv/video_color.hpp>

#include <cstdlib>

namespace {

void check(const bool condition) {
    if (!condition) std::abort();
}

} // namespace

int main() {
    using aribtlv::VideoTransferCharacteristics;
    check(aribtlv::cicp_transfer_from_b60(1) ==
          VideoTransferCharacteristics::Bt709);
    check(aribtlv::cicp_transfer_from_b60(3) ==
          VideoTransferCharacteristics::Bt2020_12);
    check(aribtlv::cicp_transfer_from_b60(4) ==
          VideoTransferCharacteristics::Smpte2084);
    check(aribtlv::cicp_transfer_from_b60(5) ==
          VideoTransferCharacteristics::AribHlg);
    check(!aribtlv::cicp_transfer_from_b60(0).has_value());
    check(aribtlv::is_pq_transfer(VideoTransferCharacteristics::Smpte2084));
    check(aribtlv::is_hlg_transfer(VideoTransferCharacteristics::AribHlg));
    check(aribtlv::is_hdr_transfer(VideoTransferCharacteristics::Smpte2084));
    check(!aribtlv::is_hdr_transfer(VideoTransferCharacteristics::Bt709));
    check(aribtlv::is_bt2020_hlg(9, 18, 9, false));
    check(!aribtlv::is_bt2020_hlg(9, 18, 9, true));
    check(aribtlv::is_bt2020_pq(9, 16, 9, false));
    check(!aribtlv::is_bt2020_pq(9, 18, 9, false));
}
