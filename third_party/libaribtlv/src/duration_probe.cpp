#include <aribtlv/asset_layers.hpp>
#include <aribtlv/duration_probe.hpp>

#include <aribtlv/demuxer.hpp>

#include "duration_probe_range.hpp"

#include <algorithm>
#include <limits>
#include <string>
#include <utility>
#include <vector>

namespace aribtlv {
namespace {

constexpr std::uint32_t microsecond_timescale = 1000000;

bool timestamp_microseconds(const Timestamp timestamp, std::int64_t& output) noexcept {
    if (timestamp.timescale == 0) return false;
    const auto scale = static_cast<std::int64_t>(timestamp.timescale);
    const auto whole = timestamp.value / scale;
    const auto remainder = timestamp.value % scale;
    constexpr auto factor = static_cast<std::int64_t>(microsecond_timescale);
    if (whole > std::numeric_limits<std::int64_t>::max() / factor ||
        whole < std::numeric_limits<std::int64_t>::min() / factor) {
        return false;
    }
    const auto scaled_whole = whole * factor;
    const auto fractional = remainder * factor / scale;
    if ((fractional > 0 && scaled_whole > std::numeric_limits<std::int64_t>::max() - fractional) ||
        (fractional < 0 && scaled_whole < std::numeric_limits<std::int64_t>::min() - fractional)) {
        return false;
    }
    output = scaled_whole + fractional;
    return true;
}

std::optional<std::int64_t> timestamp_distance(const std::int64_t first,
                                               const std::int64_t second) noexcept {
    if (first == second) return std::nullopt;
    const auto difference = first > second
        ? static_cast<std::uint64_t>(first) - static_cast<std::uint64_t>(second)
        : static_cast<std::uint64_t>(second) - static_cast<std::uint64_t>(first);
    if (difference > static_cast<std::uint64_t>(std::numeric_limits<std::int64_t>::max())) {
        return std::nullopt;
    }
    return static_cast<std::int64_t>(difference);
}

} // namespace

class DurationProbe::Impl final : public Sink {
public:
    Impl() : demuxer_(*this) {}

    bool begin(const std::uint64_t source_size, DurationProbeOptions options) {
        ++generation_;
        if (generation_ == 0) ++generation_;
        state_ = DurationProbeState::Idle;
        failure_ = DurationProbeFailure::None;
        duration_ = {};
        presentation_start_.reset();
        presentation_end_.reset();
        source_size_ = source_size;
        options_ = std::move(options);
        transferred_bytes_ = 0;
        request_.reset();
        request_received_ = 0;
        head_end_ = 0;
        tail_window_ = 0;
        candidates_.clear();
        reset_tail_statistics();
        selected_video_packet_id_.reset();
        presentation_end_video_packet_id_.reset();
        demuxer_.reset();

        if (source_size_ == 0 || options_.initial_range_size == 0 ||
            options_.max_range_size < options_.initial_range_size) {
            unknown(DurationProbeFailure::InvalidSource);
            return false;
        }
        demuxer_.selectService(options_.service_context_id);
        phase_ = Phase::Head;
        issue_request(0, std::min(source_size_, options_.initial_range_size));
        return true;
    }

    std::optional<RangeRequest> next_range() const noexcept {
        if (state_ != DurationProbeState::NeedRange) return std::nullopt;
        return request_;
    }

    bool push_range(const std::uint64_t request_id, const std::uint64_t absolute_offset,
                    const std::uint8_t* data, const std::size_t size,
                    const bool end_of_range) {
        if (state_ != DurationProbeState::NeedRange || !request_.has_value() ||
            request_->request_id != request_id) {
            return false;
        }
        const auto expected_offset = request_->offset + request_received_;
        const auto remaining = request_->length - request_received_;
        if (absolute_offset != expected_offset || size > remaining ||
            (data == nullptr && size != 0)) {
            fail(DurationProbeFailure::InvalidResponse);
            return false;
        }

        if (size != 0) demuxer_.push(data, size);
        if (state_ != DurationProbeState::NeedRange) return false;
        request_received_ += static_cast<std::uint64_t>(size);
        transferred_bytes_ += static_cast<std::uint64_t>(size);
        if (!end_of_range) return true;
        if (request_received_ != request_->length) {
            fail(DurationProbeFailure::InvalidResponse);
            return false;
        }
        finish_request();
        return true;
    }

    bool fail_range(const std::uint64_t request_id) {
        if (state_ != DurationProbeState::NeedRange || !request_.has_value() ||
            request_->request_id != request_id) {
            return false;
        }
        fail(DurationProbeFailure::SourceError);
        return true;
    }

    void cancel() noexcept {
        if (state_ == DurationProbeState::Complete || state_ == DurationProbeState::Unknown ||
            state_ == DurationProbeState::Failed) {
            return;
        }
        ++generation_;
        if (generation_ == 0) ++generation_;
        request_.reset();
        state_ = DurationProbeState::Cancelled;
        failure_ = DurationProbeFailure::None;
    }

    DurationProbeState state() const noexcept { return state_; }
    DurationProbeFailure failure() const noexcept { return failure_; }
    DurationInfo duration() const noexcept { return duration_; }
    std::optional<Timestamp> presentation_start() const noexcept {
        return presentation_start_;
    }
    std::optional<Timestamp> presentation_end() const noexcept {
        return presentation_end_;
    }
    std::optional<std::uint16_t> selected_video_packet_id() const noexcept {
        return selected_video_packet_id_;
    }
    std::optional<std::uint16_t> presentation_end_video_packet_id() const noexcept {
        return presentation_end_video_packet_id_;
    }
    std::uint64_t generation() const noexcept { return generation_; }
    std::uint64_t transferred_bytes() const noexcept { return transferred_bytes_; }

    void onService(const ServiceInfo&) override {}

    void onTrack(const TrackInfo& info) override {
        if (info.kind != TrackKind::Video || info.codec != Codec::Hevc) return;
        if (options_.video_packet_id.has_value() &&
            info.packet_id != *options_.video_packet_id) {
            return;
        }
        if (std::find_if(candidates_.begin(), candidates_.end(),
                         [&info](const VideoCandidate& candidate) {
                             return candidate.info.track_id == info.track_id;
                         }) != candidates_.end()) return;
        VideoCandidate candidate;
        candidate.info = info;
        candidates_.push_back(std::move(candidate));
        if (options_.video_packet_id.has_value()) {
            demuxer_.selectTrack(TrackKind::Video, info.track_id);
        }
    }

    void onAccessUnit(AccessUnit&& unit) override {
        if (unit.codec != Codec::Hevc) return;
        const auto found = std::find_if(
            candidates_.begin(), candidates_.end(),
            [&unit](const VideoCandidate& candidate) {
                return candidate.info.track_id == unit.track_id;
            });
        if (found == candidates_.end()) return;
        std::int64_t pts_us = 0;
        if (!timestamp_microseconds(unit.pts, pts_us)) return;
        if (phase_ == Phase::Head || phase_ == Phase::SequentialTail) {
            ++found->head_count;
            if (!found->head_first_pts_us.has_value()) found->head_first_pts_us = pts_us;
            observe_timestamp(pts_us, found->head_previous_pts_us,
                              found->head_maximum_pts_us,
                              found->head_frame_duration_us);
        } else if (phase_ == Phase::Tail) {
            ++found->tail_count;
            observe_timestamp(pts_us, found->tail_previous_pts_us,
                              found->tail_maximum_pts_us,
                              found->tail_frame_duration_us);
        }
    }

    void onError(const Error& error) override {
        if (!error.recoverable && state_ == DurationProbeState::NeedRange) {
            fail(DurationProbeFailure::ParseError);
        }
    }

private:
    enum class Phase { Head, SequentialTail, Tail };

    static void observe_timestamp(const std::int64_t pts_us,
                                  std::optional<std::int64_t>& previous,
                                  std::optional<std::int64_t>& maximum,
                                  std::int64_t& frame_duration) noexcept {
        if (previous.has_value()) {
            const auto distance = timestamp_distance(pts_us, *previous);
            if (distance.has_value() &&
                (frame_duration == 0 || *distance < frame_duration)) {
                frame_duration = *distance;
            }
        }
        previous = pts_us;
        if (!maximum.has_value() || pts_us > *maximum) maximum = pts_us;
    }

    void issue_request(const std::uint64_t offset, const std::uint64_t length) {
        ++next_request_id_;
        if (next_request_id_ == 0) ++next_request_id_;
        request_ = RangeRequest{generation_, next_request_id_, offset, length};
        request_received_ = 0;
        state_ = DurationProbeState::NeedRange;
    }

    void finish_request() {
        const auto completed = *request_;
        request_.reset();
        if (phase_ == Phase::Head) {
            head_end_ = completed.offset + completed.length;
            finish_head_range();
            return;
        }
        if (phase_ == Phase::SequentialTail) {
            demuxer_.flush();
            if (state_ != DurationProbeState::NeedRange) return;
            complete_from_candidates(false);
            return;
        }

        demuxer_.flush();
        if (state_ != DurationProbeState::NeedRange) return;
        if (has_complete_candidate(true)) {
            complete_from_candidates(true);
            return;
        }
        widen_tail();
    }

    void finish_head_range() {
        if (has_head_video()) {
            if (head_end_ == source_size_) {
                demuxer_.flush();
                if (state_ != DurationProbeState::NeedRange) return;
                complete_from_candidates(false);
                return;
            }
            const auto remaining = source_size_ - head_end_;
            if (remaining <= options_.initial_range_size) {
                phase_ = Phase::SequentialTail;
                issue_request(head_end_, remaining);
                return;
            }
            tail_window_ = std::min(options_.initial_range_size, source_size_);
            issue_tail_request();
            return;
        }

        const auto current_window = head_end_;
        const auto doubled = current_window > options_.max_range_size / 2
            ? options_.max_range_size
            : current_window * 2;
        const auto target = std::min(source_size_, std::min(options_.max_range_size, doubled));
        if (target <= head_end_) {
            unknown(DurationProbeFailure::NoVideo);
            return;
        }
        issue_request(head_end_, target - head_end_);
    }

    void issue_tail_request() {
        reset_tail_statistics();
        const auto offset = source_size_ - std::min(source_size_, tail_window_);
        demuxer_.reposition(RepositionOptions{offset, true});
        phase_ = Phase::Tail;
        issue_request(offset, source_size_ - offset);
    }

    void widen_tail() {
        if (tail_window_ >= options_.max_range_size || tail_window_ >= source_size_) {
            unknown(!has_tail_video()
                        ? DurationProbeFailure::NoTailTimestamp
                        : DurationProbeFailure::RangeLimit);
            return;
        }
        tail_window_ = tail_window_ > options_.max_range_size / 2
            ? options_.max_range_size
            : std::min(options_.max_range_size, tail_window_ * 2);
        issue_tail_request();
    }

    void complete_from_range(
        const std::optional<detail::VideoPresentationRange> range) {
        if (!range.has_value()) {
            unknown(DurationProbeFailure::NoTailTimestamp);
            return;
        }
        presentation_start_ = Timestamp{range->start_us, microsecond_timescale};
        presentation_end_ = Timestamp{range->end_us, microsecond_timescale};
        duration_ = DurationInfo{Timestamp{range->end_us - range->start_us,
                                           microsecond_timescale},
                                 DurationStatus::Complete};
        state_ = DurationProbeState::Complete;
        failure_ = DurationProbeFailure::None;
    }

    struct VideoCandidate {
        TrackInfo info;
        std::uint64_t head_count = 0;
        std::optional<std::int64_t> head_first_pts_us;
        std::optional<std::int64_t> head_previous_pts_us;
        std::optional<std::int64_t> head_maximum_pts_us;
        std::int64_t head_frame_duration_us = 0;
        std::uint64_t tail_count = 0;
        std::optional<std::int64_t> tail_previous_pts_us;
        std::optional<std::int64_t> tail_maximum_pts_us;
        std::int64_t tail_frame_duration_us = 0;
    };

    static unsigned selection_level(const VideoCandidate& candidate) noexcept {
        return assetSelectionLevel(candidate.info).value_or(0);
    }

    bool belongs_to_selected_video_range(const VideoCandidate& candidate) const noexcept {
        if (options_.video_packet_id.has_value()) {
            return candidate.info.packet_id == *options_.video_packet_id;
        }
        if (candidates_.empty()) return false;
        const auto grouped = std::find_if(
            candidates_.begin(), candidates_.end(),
            [](const VideoCandidate& item) { return !item.info.asset_groups.empty(); });
        if (grouped == candidates_.end()) return &candidate == &candidates_.front();
        const auto group_identification =
            grouped->info.asset_groups.front().group_identification;
        const auto implicit_base = candidate.info.asset_groups.empty() &&
            candidate.info.context_id == grouped->info.context_id;
        return implicit_base || belongsToAssetGroup(candidate.info, group_identification);
    }

    static std::optional<std::int64_t> candidate_end_us(
        const VideoCandidate& candidate, const bool tail) noexcept {
        const auto count = tail ? candidate.tail_count : candidate.head_count;
        const auto maximum = tail ? candidate.tail_maximum_pts_us
                                  : candidate.head_maximum_pts_us;
        const auto frame_duration = tail ? candidate.tail_frame_duration_us
                                         : candidate.head_frame_duration_us;
        if (count < 2 || !maximum.has_value() || frame_duration <= 0 ||
            *maximum > std::numeric_limits<std::int64_t>::max() - frame_duration) {
            return std::nullopt;
        }
        return *maximum + frame_duration;
    }

    const VideoCandidate* choose_candidate(const bool tail) const noexcept {
        const auto has_stats = [tail](const VideoCandidate& candidate) {
            const auto count = tail ? candidate.tail_count : candidate.head_count;
            const auto& maximum = tail ? candidate.tail_maximum_pts_us
                                       : candidate.head_maximum_pts_us;
            const auto frame_duration = tail ? candidate.tail_frame_duration_us
                                             : candidate.head_frame_duration_us;
            return count >= 2 && maximum.has_value() && frame_duration > 0;
        };
        const auto duration = [tail](const VideoCandidate& candidate) {
            const auto maximum = tail ? candidate.tail_maximum_pts_us
                                      : candidate.head_maximum_pts_us;
            const auto frame_duration = tail ? candidate.tail_frame_duration_us
                                             : candidate.head_frame_duration_us;
            return maximum.value() + frame_duration;
        };
        const auto better = [&duration](const VideoCandidate* left,
                                        const VideoCandidate* right) {
            if (right == nullptr) return true;
            const auto left_duration = duration(*left);
            const auto right_duration = duration(*right);
            if (left_duration != right_duration) return left_duration > right_duration;
            const auto left_level = selection_level(*left);
            const auto right_level = selection_level(*right);
            return left_level < right_level;
        };

        const VideoCandidate* selected = nullptr;
        for (const auto& candidate : candidates_) {
            if (belongs_to_selected_video_range(candidate) && has_stats(candidate) &&
                better(&candidate, selected)) {
                selected = &candidate;
            }
        }
        return selected;
    }

    const VideoCandidate* choose_start_candidate() const noexcept {
        const VideoCandidate* selected = nullptr;
        for (const auto& candidate : candidates_) {
            if (!belongs_to_selected_video_range(candidate) ||
                !candidate.head_first_pts_us.has_value()) continue;
            if (selected == nullptr ||
                *candidate.head_first_pts_us < *selected->head_first_pts_us ||
                (*candidate.head_first_pts_us == *selected->head_first_pts_us &&
                 selection_level(candidate) < selection_level(*selected))) {
                selected = &candidate;
            }
        }
        return selected;
    }

    bool has_head_video() const noexcept {
        return std::any_of(candidates_.begin(), candidates_.end(),
                           [](const VideoCandidate& candidate) {
                               return candidate.head_count != 0;
                           });
    }

    bool has_complete_candidate(const bool tail) const noexcept {
        return choose_candidate(tail) != nullptr;
    }

    void complete_from_candidates(const bool tail) {
        const auto* candidate = choose_candidate(tail);
        if (candidate == nullptr) {
            unknown(tail ? DurationProbeFailure::NoTailTimestamp
                         : DurationProbeFailure::NoVideo);
            return;
        }
        std::vector<detail::VideoPresentationBoundary> boundaries;
        for (const auto& item : candidates_) {
            if (!belongs_to_selected_video_range(item)) continue;
            boundaries.push_back(detail::VideoPresentationBoundary{
                item.head_first_pts_us, candidate_end_us(item, tail)});
        }
        complete_from_range(detail::union_video_presentation_ranges(boundaries));
        if (state_ == DurationProbeState::Complete) {
            const auto* start_candidate = choose_start_candidate();
            selected_video_packet_id_ = start_candidate != nullptr
                ? std::optional<std::uint16_t>{start_candidate->info.packet_id}
                : std::optional<std::uint16_t>{candidate->info.packet_id};
            presentation_end_video_packet_id_ = candidate->info.packet_id;
        }
    }

    void reset_tail_statistics() noexcept {
        for (auto& candidate : candidates_) {
            candidate.tail_count = 0;
            candidate.tail_previous_pts_us.reset();
            candidate.tail_maximum_pts_us.reset();
            candidate.tail_frame_duration_us = 0;
        }
    }

    bool has_tail_video() const noexcept {
        return std::any_of(candidates_.begin(), candidates_.end(),
                           [](const VideoCandidate& candidate) {
                               return candidate.tail_count != 0;
                           });
    }

    void unknown(const DurationProbeFailure failure) noexcept {
        request_.reset();
        duration_ = {};
        presentation_start_.reset();
        presentation_end_.reset();
        state_ = DurationProbeState::Unknown;
        failure_ = failure;
    }

    void fail(const DurationProbeFailure failure) noexcept {
        request_.reset();
        duration_ = {};
        presentation_start_.reset();
        presentation_end_.reset();
        state_ = DurationProbeState::Failed;
        failure_ = failure;
    }

    Demuxer demuxer_;
    DurationProbeOptions options_;
    DurationProbeState state_ = DurationProbeState::Idle;
    DurationProbeFailure failure_ = DurationProbeFailure::None;
    DurationInfo duration_;
    std::optional<Timestamp> presentation_start_;
    std::optional<Timestamp> presentation_end_;
    Phase phase_ = Phase::Head;
    std::uint64_t source_size_ = 0;
    std::uint64_t generation_ = 0;
    std::uint64_t next_request_id_ = 0;
    std::optional<RangeRequest> request_;
    std::uint64_t request_received_ = 0;
    std::uint64_t transferred_bytes_ = 0;
    std::uint64_t head_end_ = 0;
    std::uint64_t tail_window_ = 0;
    std::vector<VideoCandidate> candidates_;
    std::optional<std::uint16_t> selected_video_packet_id_;
    std::optional<std::uint16_t> presentation_end_video_packet_id_;
};

DurationProbe::DurationProbe() : impl_(std::make_unique<Impl>()) {}
DurationProbe::~DurationProbe() = default;
DurationProbe::DurationProbe(DurationProbe&&) noexcept = default;
DurationProbe& DurationProbe::operator=(DurationProbe&&) noexcept = default;

bool DurationProbe::begin(const std::uint64_t source_size, DurationProbeOptions options) {
    return impl_->begin(source_size, std::move(options));
}

std::optional<RangeRequest> DurationProbe::nextRange() const noexcept {
    return impl_->next_range();
}

bool DurationProbe::pushRange(const std::uint64_t request_id,
                              const std::uint64_t absolute_offset,
                              const std::uint8_t* data, const std::size_t size,
                              const bool end_of_range) {
    return impl_->push_range(request_id, absolute_offset, data, size, end_of_range);
}

bool DurationProbe::failRange(const std::uint64_t request_id) {
    return impl_->fail_range(request_id);
}

void DurationProbe::cancel() noexcept { impl_->cancel(); }
DurationProbeState DurationProbe::state() const noexcept { return impl_->state(); }
DurationProbeFailure DurationProbe::failure() const noexcept { return impl_->failure(); }
DurationInfo DurationProbe::duration() const noexcept { return impl_->duration(); }
std::optional<Timestamp> DurationProbe::presentationStart() const noexcept {
    return impl_->presentation_start();
}
std::optional<Timestamp> DurationProbe::presentationEnd() const noexcept {
    return impl_->presentation_end();
}
std::optional<std::uint16_t> DurationProbe::selectedVideoPacketId() const noexcept {
    return impl_->selected_video_packet_id();
}
std::optional<std::uint16_t> DurationProbe::presentationEndVideoPacketId() const noexcept {
    return impl_->presentation_end_video_packet_id();
}
std::uint64_t DurationProbe::generation() const noexcept { return impl_->generation(); }
std::uint64_t DurationProbe::transferredBytes() const noexcept {
    return impl_->transferred_bytes();
}

} // namespace aribtlv
