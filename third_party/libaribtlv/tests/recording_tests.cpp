#include <aribtlv/recording.hpp>
#include <aribtlv/duration_probe.hpp>

#include "../src/duration_probe_range.hpp"

#include <cstdint>
#include <cstdlib>
#include <iostream>
#include <string>

namespace {

[[noreturn]] void fail(const std::string& message) {
    std::cerr << "FAIL: " << message << '\n';
    std::exit(1);
}

void check(const bool condition, const std::string& message) {
    if (!condition) fail(message);
}

aribtlv::AccessUnit video_unit(const std::int64_t pts_us,
                                const std::uint64_t input_offset,
                                const std::uint64_t restart_offset,
                                const bool random_access) {
    aribtlv::AccessUnit unit;
    unit.track_id = 7;
    unit.codec = aribtlv::Codec::Hevc;
    unit.pts = aribtlv::Timestamp{pts_us, 1000000};
    unit.dts = unit.pts;
    unit.input_offset = input_offset;
    unit.restart_offset = restart_offset;
    unit.random_access = random_access;
    return unit;
}

void test_recording_index() {
    aribtlv::RecordingIndex index;
    index.begin(false);
    check(index.state() == aribtlv::IndexState::Building,
          "recording index did not enter Building");

    check(index.observe(video_unit(0, 100, 20, true)), "first video AU was rejected");
    check(index.observe(video_unit(1000000, 1100, 20, false)), "second video AU was rejected");
    check(index.observe(video_unit(2000000, 2100, 60, true)), "second RAP was rejected");
    check(index.observe(video_unit(3000000, 3100, 60, false)), "tail video AU was rejected");

    const auto provisional = index.duration();
    check(provisional.status == aribtlv::DurationStatus::Provisional &&
              provisional.value.value == 3000000,
          "indexed presentation extent did not produce provisional duration");
    check(index.seekPoints().size() == 2,
          "random-access AUs did not produce two seek points");
    const auto before_first_rap = index.seekPointsFor(aribtlv::Timestamp{-1, 1});
    check(before_first_rap.has_value() &&
              before_first_rap->first.presentation_time.value == 0,
          "seek map did not expose its first decodable RAP before the indexed range");

    const auto previous = index.previousSync(aribtlv::Timestamp{2500, 1000});
    check(previous.has_value() && previous->presentation_time.value == 2000000 &&
              previous->signalling_offset == 60 &&
              previous->random_access_offset == 2100,
          "previous-sync lookup selected the wrong RAP");
    const auto surrounding = index.seekPointsFor(aribtlv::Timestamp{1, 1});
    check(surrounding.has_value() &&
              surrounding->first.presentation_time.value == 0 &&
              surrounding->second.has_value() &&
              surrounding->second->presentation_time.value == 2000000,
          "two-sided seek map lookup did not return surrounding RAPs");

    const auto middle_estimate = index.estimateOffset(aribtlv::Timestamp{1, 1}, 4100);
    check(middle_estimate.has_value() && *middle_estimate == 1100,
          "piecewise seek interpolation produced the wrong middle offset");

    check(!index.addSeekPoint(aribtlv::SeekPoint{
              aribtlv::Timestamp{5, 1}, 5000, 4000, 7, 0}),
          "seek point with signalling after RAP was accepted");
    check(index.finalize(), "recording index did not finalize");
    check(index.duration().status == aribtlv::DurationStatus::Complete &&
              index.duration().value.value == 4000000,
          "finalized recording duration did not include inferred tail frame duration");
    const auto tail_estimate = index.estimateOffset(aribtlv::Timestamp{3, 1}, 4100);
    check(tail_estimate.has_value() && *tail_estimate == 3100,
          "tail seek interpolation did not use duration and source size");
    check(!index.updateDuration(aribtlv::DurationInfo{
              aribtlv::Timestamp{3, 1}, aribtlv::DurationStatus::Complete}),
          "complete duration was allowed to decrease");
}

void test_duration_uses_recording_timeline_endpoint() {
    aribtlv::RecordingIndex index;
    index.begin(false);
    check(index.observe(video_unit(500000, 100, 20, true)) &&
              index.observe(video_unit(1500000, 1100, 20, false)) &&
              index.observe(video_unit(2500000, 2100, 60, true)),
          "non-zero-origin video AUs were rejected");
    check(index.duration().status == aribtlv::DurationStatus::Provisional &&
              index.duration().value.value == 2500000,
          "provisional duration subtracted the first video PTS from the timeline endpoint");
    check(index.finalize() && index.duration().value.value == 3500000,
          "complete duration did not include the final frame interval on the recording timeline");
}

void test_duration_probe_range_protocol() {
    aribtlv::DurationProbe probe;
    aribtlv::DurationProbeOptions options;
    options.initial_range_size = 10;
    options.max_range_size = 40;
    check(probe.begin(100, options), "duration probe rejected a valid source");

    auto request = probe.nextRange();
    check(request.has_value() && request->offset == 0 && request->length == 10,
          "duration probe did not request its initial head range");
    const auto generation = request->generation;
    const std::vector<std::uint8_t> garbage(40, 0x55);
    check(!probe.pushRange(request->request_id + 1, 0, garbage.data(), 10, true) &&
              probe.state() == aribtlv::DurationProbeState::NeedRange,
          "stale duration response changed the active request");
    check(probe.pushRange(request->request_id, 0, garbage.data(), 4, false) &&
              probe.pushRange(request->request_id, 4, garbage.data() + 4, 6, true),
          "chunked duration response was rejected");

    request = probe.nextRange();
    check(request.has_value() && request->generation == generation &&
              request->offset == 10 && request->length == 10,
          "duration probe did not double its head window");
    check(probe.pushRange(request->request_id, 10, garbage.data(), 10, true),
          "second duration range was rejected");
    request = probe.nextRange();
    check(request.has_value() && request->offset == 20 && request->length == 20,
          "duration probe did not reach its configured maximum head window");
    check(probe.pushRange(request->request_id, 20, garbage.data(), 20, true) &&
              probe.state() == aribtlv::DurationProbeState::Unknown &&
              probe.failure() == aribtlv::DurationProbeFailure::NoVideo &&
              probe.transferredBytes() == 40,
          "duration probe did not stop after its no-video byte budget");

    check(probe.begin(100, options), "duration probe could not restart");
    request = probe.nextRange();
    check(request.has_value(), "restarted duration probe has no range request");
    const auto old_request = *request;
    probe.cancel();
    check(probe.state() == aribtlv::DurationProbeState::Cancelled &&
              !probe.pushRange(old_request.request_id, old_request.offset,
                               garbage.data(), 10, true),
          "cancelled duration probe accepted a late response");

    check(probe.begin(100, options), "duration probe could not restart after cancel");
    request = probe.nextRange();
    check(request.has_value() && probe.failRange(request->request_id) &&
              probe.state() == aribtlv::DurationProbeState::Failed &&
              probe.failure() == aribtlv::DurationProbeFailure::SourceError,
          "duration probe did not preserve a source failure");

    check(probe.begin(100, options), "duration probe could not restart after failure");
    request = probe.nextRange();
    check(request.has_value() &&
              !probe.pushRange(request->request_id, request->offset,
                               garbage.data(), 5, true) &&
              probe.failure() == aribtlv::DurationProbeFailure::InvalidResponse,
          "duration probe accepted a short completed range");

    check(!probe.begin(0, options) &&
              probe.state() == aribtlv::DurationProbeState::Unknown &&
              probe.failure() == aribtlv::DurationProbeFailure::InvalidSource,
          "duration probe accepted a zero-sized source");
}

void test_dual_video_presentation_range_union() {
    using aribtlv::detail::VideoPresentationBoundary;
    using aribtlv::detail::union_video_presentation_ranges;
    const auto check_union = [](const std::vector<VideoPresentationBoundary>& tracks,
                                const std::int64_t start_us,
                                const std::int64_t end_us,
                                const char* message) {
        const auto range = union_video_presentation_ranges(tracks);
        check(range.has_value() && range->start_us == start_us &&
                  range->end_us == end_us &&
                  range->end_us - range->start_us == end_us - start_us,
              message);
    };
    check_union({{200000, 9000000}, {100000, 8000000}}, 100000, 9000000,
                "rainfall-first union lost the earlier start");
    check_union({{100000, 8000000}, {200000, 9000000}}, 100000, 9000000,
                "preferred-first union lost the earlier start");
    check_union({{100000, 9000000}, {200000, 8000000}}, 100000, 9000000,
                "preferred-later union selected the wrong end");
    check_union({{100000, 8000000}, {200000, 9000000}}, 100000, 9000000,
                "rainfall-later union selected the wrong end");
}

void test_recording_scanner_failure_contract() {
    aribtlv::RecordingScanner empty;
    check(empty.finish().failure == aribtlv::RecordingScanFailure::NoVideo,
          "empty recording scan did not report NoVideo");

    aribtlv::RecordingScanner source_failure;
    source_failure.failSource();
    check(source_failure.finish().failure == aribtlv::RecordingScanFailure::SourceError,
          "recording scan did not preserve its source failure");
    check(!source_failure.push(nullptr, 0),
          "finished recording scan accepted more input");
}

} // namespace

int main() {
    test_recording_index();
    test_duration_uses_recording_timeline_endpoint();
    test_duration_probe_range_protocol();
    test_dual_video_presentation_range_union();
    test_recording_scanner_failure_contract();
    std::cout << "all recording tests passed\n";
    return 0;
}
