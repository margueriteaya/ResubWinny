#include <aribtlv/demuxer.hpp>
#include <aribtlv/recording.hpp>

namespace {

class NullSink final : public aribtlv::Sink {
public:
    void onService(const aribtlv::ServiceInfo&) override {}
    void onTrack(const aribtlv::TrackInfo&) override {}
    void onAccessUnit(aribtlv::AccessUnit&&) override {}
    void onError(const aribtlv::Error&) override {}
};

} // namespace

int main() {
    NullSink sink;
    aribtlv::Demuxer demuxer(sink);
    aribtlv::RecordingIndex index;
    index.begin(false);
    demuxer.flush();
    return 0;
}
