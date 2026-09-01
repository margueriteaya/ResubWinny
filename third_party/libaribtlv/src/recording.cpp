#include <aribtlv/recording.hpp>
#include <aribtlv/demuxer.hpp>

#include <algorithm>
#include <iterator>
#include <limits>
#include <utility>

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
    const auto fractional = remainder * factor / scale;
    const auto scaled_whole = whole * factor;
    if ((fractional > 0 && scaled_whole > std::numeric_limits<std::int64_t>::max() - fractional) ||
        (fractional < 0 && scaled_whole < std::numeric_limits<std::int64_t>::min() - fractional)) {
        return false;
    }
    output = scaled_whole + fractional;
    return true;
}

std::uint64_t interpolate_offset(const std::uint64_t first_offset,
                                 const std::uint64_t second_offset,
                                 const std::int64_t first_time,
                                 const std::int64_t second_time,
                                 const std::int64_t target_time) noexcept {
    if (second_time <= first_time || second_offset <= first_offset || target_time <= first_time) {
        return first_offset;
    }
    if (target_time >= second_time) return second_offset;
    const auto numerator = static_cast<long double>(target_time - first_time);
    const auto denominator = static_cast<long double>(second_time - first_time);
    const auto byte_delta = static_cast<long double>(second_offset - first_offset);
    const auto interpolated = static_cast<long double>(first_offset) + byte_delta * numerator / denominator;
    if (interpolated <= static_cast<long double>(first_offset)) return first_offset;
    if (interpolated >= static_cast<long double>(second_offset)) return second_offset;
    return static_cast<std::uint64_t>(interpolated);
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

void RecordingIndex::reset() {
    state_ = IndexState::Absent;
    selected_video_track_.reset();
    seek_points_.clear();
    minimum_pts_us_.reset();
    maximum_pts_us_.reset();
    previous_pts_us_.reset();
    inferred_frame_duration_us_ = 0;
    duration_us_.reset();
    duration_status_ = DurationStatus::Unknown;
    growing_ = false;
}

void RecordingIndex::begin(const bool growing) {
    reset();
    growing_ = growing;
    state_ = IndexState::Building;
}

void RecordingIndex::selectVideoTrack(std::optional<std::uint64_t> track_id) {
    if (selected_video_track_ == track_id) return;
    selected_video_track_ = track_id;
    seek_points_.clear();
    minimum_pts_us_.reset();
    maximum_pts_us_.reset();
    previous_pts_us_.reset();
    inferred_frame_duration_us_ = 0;
    duration_us_.reset();
    duration_status_ = DurationStatus::Unknown;
}

void RecordingIndex::switchVideoTrack(const std::uint64_t track_id) {
    if (selected_video_track_ == track_id) return;
    selected_video_track_ = track_id;
    previous_pts_us_.reset();
    inferred_frame_duration_us_ = 0;
}

bool RecordingIndex::observe(const AccessUnit& unit) {
    if (state_ != IndexState::Building || unit.codec != Codec::Hevc) return false;
    if (!selected_video_track_.has_value()) selected_video_track_ = unit.track_id;
    if (*selected_video_track_ != unit.track_id) return false;

    std::int64_t pts_us = 0;
    if (!timestamp_microseconds(unit.pts, pts_us)) return false;
    if (previous_pts_us_.has_value()) {
        const auto distance = timestamp_distance(pts_us, *previous_pts_us_);
        if (distance.has_value() &&
            (inferred_frame_duration_us_ == 0 || *distance < inferred_frame_duration_us_)) {
            inferred_frame_duration_us_ = *distance;
        }
    }
    previous_pts_us_ = pts_us;
    if (!minimum_pts_us_.has_value() || pts_us < *minimum_pts_us_) minimum_pts_us_ = pts_us;
    if (!maximum_pts_us_.has_value()) {
        maximum_pts_us_ = pts_us;
    } else if (pts_us > *maximum_pts_us_) {
        maximum_pts_us_ = pts_us;
    }

    if (maximum_pts_us_.has_value()) {
        const auto candidate = std::max<std::int64_t>(0, *maximum_pts_us_);
        if (!duration_us_.has_value() || candidate > *duration_us_) {
            duration_us_ = candidate;
        }
    }
    update_duration_status();

    if (!unit.random_access) return true;
    return addSeekPoint(SeekPoint{
        Timestamp{pts_us, microsecond_timescale},
        unit.restart_offset,
        unit.input_offset,
        unit.track_id,
        0,
    });
}

bool RecordingIndex::addSeekPoint(SeekPoint point) {
    std::int64_t time_us = 0;
    if (state_ != IndexState::Building || point.video_track_id == 0 ||
        point.signalling_offset > point.random_access_offset ||
        !timestamp_microseconds(point.presentation_time, time_us)) {
        return false;
    }
    if (!selected_video_track_.has_value()) {
        selected_video_track_ = point.video_track_id;
    } else if (*selected_video_track_ != point.video_track_id) {
        return false;
    }
    point.presentation_time = Timestamp{time_us, microsecond_timescale};
    const auto found = std::lower_bound(
        seek_points_.begin(), seek_points_.end(), point,
        [](const SeekPoint& left, const SeekPoint& right) {
            if (left.presentation_time.value != right.presentation_time.value) {
                return left.presentation_time.value < right.presentation_time.value;
            }
            return left.random_access_offset < right.random_access_offset;
        });
    if (found != seek_points_.end() &&
        found->presentation_time.value == point.presentation_time.value &&
        found->random_access_offset == point.random_access_offset &&
        found->video_track_id == point.video_track_id) {
        return true;
    }
    if (found != seek_points_.begin()) {
        const auto& previous = *std::prev(found);
        if (previous.random_access_offset > point.random_access_offset) return false;
    }
    if (found != seek_points_.end() &&
        point.random_access_offset > found->random_access_offset) {
        return false;
    }
    seek_points_.insert(found, std::move(point));
    return true;
}

bool RecordingIndex::updateDuration(DurationInfo info) {
    std::int64_t value_us = 0;
    if (info.status == DurationStatus::Unknown ||
        !timestamp_microseconds(info.value, value_us) || value_us < 0) {
        return false;
    }
    if ((info.status == DurationStatus::Complete) != (state_ == IndexState::Complete)) {
        return false;
    }
    if (duration_status_ == DurationStatus::Complete &&
        (!duration_us_.has_value() || value_us != *duration_us_)) {
        return false;
    }
    if (duration_us_.has_value() && value_us < *duration_us_) return false;
    duration_us_ = value_us;
    duration_status_ = info.status;
    return true;
}

bool RecordingIndex::pause() {
    if (state_ != IndexState::Building) return false;
    state_ = IndexState::Partial;
    update_duration_status();
    return true;
}

bool RecordingIndex::resume() {
    if (state_ != IndexState::Partial && state_ != IndexState::Following &&
        state_ != IndexState::Stale && state_ != IndexState::Failed) {
        return false;
    }
    state_ = IndexState::Building;
    return true;
}

bool RecordingIndex::reachReadableEnd(const bool growing) {
    if (state_ != IndexState::Building) return false;
    growing_ = growing;
    state_ = growing ? IndexState::Following : IndexState::Complete;
    update_duration_status();
    return true;
}

bool RecordingIndex::finalize() {
    if (state_ != IndexState::Building && state_ != IndexState::Partial &&
        state_ != IndexState::Following) {
        return false;
    }
    state_ = IndexState::Complete;
    growing_ = false;
    update_duration_status();
    return true;
}

void RecordingIndex::markStale() {
    state_ = IndexState::Stale;
    seek_points_.clear();
    minimum_pts_us_.reset();
    maximum_pts_us_.reset();
    previous_pts_us_.reset();
    inferred_frame_duration_us_ = 0;
    duration_us_.reset();
    duration_status_ = DurationStatus::Unknown;
}

void RecordingIndex::fail() {
    state_ = IndexState::Failed;
    update_duration_status();
}

std::optional<SeekPoint> RecordingIndex::previousSync(const Timestamp target) const {
    std::int64_t target_us = 0;
    if (seek_points_.empty() || !timestamp_microseconds(target, target_us)) return std::nullopt;
    const auto found = std::upper_bound(
        seek_points_.begin(), seek_points_.end(), target_us,
        [](const std::int64_t value, const SeekPoint& point) {
            return value < point.presentation_time.value;
        });
    if (found == seek_points_.begin()) return std::nullopt;
    return *std::prev(found);
}

std::optional<SeekPoints> RecordingIndex::seekPointsFor(const Timestamp target) const {
    std::int64_t target_us = 0;
    if (seek_points_.empty() || !timestamp_microseconds(target, target_us)) return std::nullopt;
    const auto upper = std::upper_bound(
        seek_points_.begin(), seek_points_.end(), target_us,
        [](const std::int64_t value, const SeekPoint& point) {
            return value < point.presentation_time.value;
        });
    if (upper == seek_points_.begin()) return SeekPoints{*upper, std::nullopt};
    const auto previous = std::prev(upper);
    if (upper == seek_points_.end() || previous->presentation_time.value == target_us) {
        return SeekPoints{*previous, std::nullopt};
    }
    return SeekPoints{*previous, *upper};
}

std::optional<std::uint64_t> RecordingIndex::estimateOffset(
    const Timestamp target, const std::uint64_t source_size) const {
    std::int64_t target_us = 0;
    if (source_size == 0 || !timestamp_microseconds(target, target_us)) return std::nullopt;
    if (target_us <= 0) return 0;

    const auto upper = std::upper_bound(
        seek_points_.begin(), seek_points_.end(), target_us,
        [](const std::int64_t value, const SeekPoint& point) {
            return value < point.presentation_time.value;
        });
    if (upper != seek_points_.begin() && upper != seek_points_.end()) {
        const auto& lower_point = *std::prev(upper);
        return interpolate_offset(lower_point.random_access_offset,
                                  upper->random_access_offset,
                                  lower_point.presentation_time.value,
                                  upper->presentation_time.value,
                                  target_us);
    }
    if (upper != seek_points_.end()) {
        return interpolate_offset(0, upper->random_access_offset, 0,
                                  upper->presentation_time.value, target_us);
    }
    if (!seek_points_.empty() && duration_us_.has_value() && target_us <= *duration_us_) {
        const auto& lower_point = seek_points_.back();
        return interpolate_offset(lower_point.random_access_offset, source_size,
                                  lower_point.presentation_time.value,
                                  *duration_us_, target_us);
    }
    if (seek_points_.empty() && duration_us_.has_value() && *duration_us_ > 0) {
        return interpolate_offset(0, source_size, 0, *duration_us_,
                                  std::min(target_us, *duration_us_));
    }
    return std::nullopt;
}

DurationInfo RecordingIndex::duration() const noexcept {
    return DurationInfo{
        Timestamp{duration_us_.value_or(0), microsecond_timescale},
        duration_status_,
    };
}

std::optional<Timestamp> RecordingIndex::presentationStart() const noexcept {
    if (!minimum_pts_us_) return std::nullopt;
    return Timestamp{*minimum_pts_us_, microsecond_timescale};
}

std::optional<Timestamp> RecordingIndex::presentationEnd() const noexcept {
    if (!maximum_pts_us_) return std::nullopt;
    return Timestamp{*maximum_pts_us_, microsecond_timescale};
}

void RecordingIndex::update_duration_status() {
    if (state_ == IndexState::Complete && maximum_pts_us_.has_value()) {
        auto final_candidate = std::max<std::int64_t>(0, *maximum_pts_us_);
        if (inferred_frame_duration_us_ > 0 &&
            final_candidate <= std::numeric_limits<std::int64_t>::max() -
                inferred_frame_duration_us_) {
            final_candidate += inferred_frame_duration_us_;
        }
        if (final_candidate >= 0 &&
            (!duration_us_.has_value() || final_candidate > *duration_us_)) {
            duration_us_ = final_candidate;
        }
    }
    if (!duration_us_.has_value()) {
        duration_status_ = DurationStatus::Unknown;
    } else if (state_ == IndexState::Complete) {
        duration_status_ = DurationStatus::Complete;
    } else {
        duration_status_ = DurationStatus::Provisional;
    }
}

class RecordingScanner::Impl final : public Sink {
public:
    explicit Impl(RecordingScanOptions options)
        : options_(std::move(options)), demuxer_(*this, scanner_limits()) {
        index_.begin(false);
        demuxer_.selectService(options_.service_context_id);
    }

    bool push(const std::uint8_t* data, const std::size_t size) {
        if (finished_ || source_failed_ || (!data && size != 0)) return false;
        demuxer_.push(data, size);
        return !fatal_error_.has_value();
    }

    void failSource() {
        if (!finished_) source_failed_ = true;
    }

    const RecordingScanResult& finish() {
        if (finished_) return result_;
        finished_ = true;
        if (source_failed_) {
            index_.fail();
            result_.failure = RecordingScanFailure::SourceError;
            return result_;
        }

        demuxer_.flush();
        result_.error = fatal_error_;
        result_.video_track_id = video_track_id_;
        result_.video_packet_id = video_packet_id_;
        if (fatal_error_) {
            index_.fail();
            result_.failure = RecordingScanFailure::ParseError;
            return result_;
        }
        if (!video_track_id_) {
            index_.fail();
            result_.failure = RecordingScanFailure::NoVideo;
            return result_;
        }
        if (!index_.finalize()) {
            result_.failure = RecordingScanFailure::ParseError;
            return result_;
        }

        result_.first_presentation_time = index_.presentationStart();
        result_.last_presentation_time = index_.presentationEnd();
        result_.duration = index_.duration();
        result_.seek_points = index_.seekPoints();
        if (!result_.first_presentation_time || !result_.last_presentation_time) {
            result_.failure = RecordingScanFailure::NoVideo;
        } else if (result_.seek_points.empty()) {
            result_.failure = RecordingScanFailure::NoRandomAccessPoint;
        }
        return result_;
    }

    std::optional<RecordingSeekResult> seekFromStart(const Timestamp offset) const {
        if (!finished_ || !result_.complete() || !result_.first_presentation_time ||
            !result_.last_presentation_time) {
            return std::nullopt;
        }
        std::int64_t offset_us = 0;
        if (!timestamp_microseconds(offset, offset_us) || offset_us < 0) return std::nullopt;
        const auto first = result_.first_presentation_time->value;
        if (first > std::numeric_limits<std::int64_t>::max() - offset_us) {
            return std::nullopt;
        }
        const auto target = first + offset_us;
        if (target > result_.last_presentation_time->value) return std::nullopt;
        const Timestamp target_timestamp{target, microsecond_timescale};
        auto point = index_.previousSync(target_timestamp);
        if (!point && !result_.seek_points.empty()) point = result_.seek_points.front();
        if (!point) return std::nullopt;
        return RecordingSeekResult{target_timestamp, *point};
    }

    void onService(const ServiceInfo&) override {}

    void onTrack(const TrackInfo& track) override {
        if (video_track_id_ || track.kind != TrackKind::Video || track.codec != Codec::Hevc ||
            (options_.video_packet_id && track.packet_id != *options_.video_packet_id)) {
            return;
        }
        video_track_id_ = track.track_id;
        video_packet_id_ = track.packet_id;
        index_.selectVideoTrack(video_track_id_);
    }

    void onAccessUnit(AccessUnit&& unit) override {
        if (video_track_id_ == unit.track_id) index_.observe(unit);
    }

    void onError(const Error& error) override {
        if (!error.recoverable && !fatal_error_) fatal_error_ = error;
    }

private:
    static Limits scanner_limits() {
        Limits limits;
        limits.collect_application_resources = false;
        return limits;
    }

    RecordingScanOptions options_;
    RecordingIndex index_;
    Demuxer demuxer_;
    RecordingScanResult result_;
    std::optional<std::uint64_t> video_track_id_;
    std::optional<std::uint16_t> video_packet_id_;
    std::optional<Error> fatal_error_;
    bool source_failed_ = false;
    bool finished_ = false;
};

RecordingScanner::RecordingScanner(RecordingScanOptions options)
    : impl_(std::make_unique<Impl>(std::move(options))) {}

RecordingScanner::~RecordingScanner() = default;
RecordingScanner::RecordingScanner(RecordingScanner&&) noexcept = default;
RecordingScanner& RecordingScanner::operator=(RecordingScanner&&) noexcept = default;

bool RecordingScanner::push(const std::uint8_t* data, const std::size_t size) {
    return impl_->push(data, size);
}

void RecordingScanner::failSource() { impl_->failSource(); }

const RecordingScanResult& RecordingScanner::finish() { return impl_->finish(); }

std::optional<RecordingSeekResult> RecordingScanner::seekFromStart(
    const Timestamp offset) const {
    return impl_->seekFromStart(offset);
}

} // namespace aribtlv
