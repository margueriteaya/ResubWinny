#include <aribtlv/video_presentation.hpp>

namespace aribtlv {

VideoPresentationHint video_presentation_hint(const EventInfo& event) noexcept {
    return event.hdr_programme_icon ? VideoPresentationHint::Hdr
                                    : VideoPresentationHint::Unknown;
}

} // namespace aribtlv
