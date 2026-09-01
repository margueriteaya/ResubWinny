#pragma once

#include <cstdint>
#include <memory>
#include <optional>
#include <vector>

#include <aribtlv/types.hpp>

namespace aribtlv {

enum class IndexState {
    Absent,
    Loading,
    Building,
    Partial,
    Following,
    Complete,
    Stale,
    Failed,
};

enum class DurationStatus { Unknown, Provisional, Complete };
enum class SeekMode { PreviousSync, ExactFrame };

struct DurationInfo {
    Timestamp value{0, 1000000};
    DurationStatus status = DurationStatus::Unknown;
};

struct SeekPoint {
    Timestamp presentation_time{0, 1000000};
    std::uint64_t signalling_offset = 0;
    std::uint64_t random_access_offset = 0;
    std::uint64_t video_track_id = 0;
    std::uint64_t bootstrap_id = 0;
};

struct SeekPoints {
    SeekPoint first;
    std::optional<SeekPoint> second;
};

class RecordingIndex {
public:
    void reset();
    void begin(bool growing);
    void selectVideoTrack(std::optional<std::uint64_t> track_id);
    void switchVideoTrack(std::uint64_t track_id);
    bool observe(const AccessUnit&);
    bool addSeekPoint(SeekPoint);
    bool updateDuration(DurationInfo);
    bool pause();
    bool resume();
    bool reachReadableEnd(bool growing);
    bool finalize();
    void markStale();
    void fail();

    std::optional<SeekPoint> previousSync(Timestamp target) const;
    std::optional<SeekPoints> seekPointsFor(Timestamp target) const;
    std::optional<std::uint64_t> estimateOffset(Timestamp target,
                                                std::uint64_t source_size) const;

    IndexState state() const noexcept { return state_; }
    DurationInfo duration() const noexcept;
    std::optional<Timestamp> presentationStart() const noexcept;
    std::optional<Timestamp> presentationEnd() const noexcept;
    const std::vector<SeekPoint>& seekPoints() const noexcept { return seek_points_; }
    std::optional<std::uint64_t> selectedVideoTrack() const noexcept {
        return selected_video_track_;
    }
    bool growing() const noexcept { return growing_; }

private:
    void update_duration_status();

    IndexState state_ = IndexState::Absent;
    std::optional<std::uint64_t> selected_video_track_;
    std::vector<SeekPoint> seek_points_;
    std::optional<std::int64_t> minimum_pts_us_;
    std::optional<std::int64_t> maximum_pts_us_;
    std::optional<std::int64_t> previous_pts_us_;
    std::int64_t inferred_frame_duration_us_ = 0;
    std::optional<std::int64_t> duration_us_;
    DurationStatus duration_status_ = DurationStatus::Unknown;
    bool growing_ = false;
};

struct RecordingScanOptions {
    std::optional<std::uint32_t> service_context_id;
    std::optional<std::uint16_t> video_packet_id;
};

enum class RecordingScanFailure {
    None,
    SourceError,
    NoVideo,
    NoRandomAccessPoint,
    ParseError,
};

struct RecordingScanResult {
    RecordingScanFailure failure = RecordingScanFailure::None;
    std::optional<Error> error;
    std::optional<std::uint64_t> video_track_id;
    std::optional<std::uint16_t> video_packet_id;
    std::optional<Timestamp> first_presentation_time;
    std::optional<Timestamp> last_presentation_time;
    DurationInfo duration;
    std::vector<SeekPoint> seek_points;

    bool complete() const noexcept { return failure == RecordingScanFailure::None; }
};

struct RecordingSeekResult {
    Timestamp target_presentation_time{0, 1000000};
    SeekPoint point;
};

// Incrementally scans a complete recording from byte zero. Source I/O remains
// the caller's responsibility; failSource() distinguishes an incomplete read
// from a valid recording that simply contains no suitable video track.
class RecordingScanner {
public:
    explicit RecordingScanner(RecordingScanOptions options = {});
    ~RecordingScanner();

    RecordingScanner(RecordingScanner&&) noexcept;
    RecordingScanner& operator=(RecordingScanner&&) noexcept;
    RecordingScanner(const RecordingScanner&) = delete;
    RecordingScanner& operator=(const RecordingScanner&) = delete;

    bool push(const std::uint8_t* data, std::size_t size);
    void failSource();
    const RecordingScanResult& finish();
    std::optional<RecordingSeekResult> seekFromStart(Timestamp offset) const;

private:
    class Impl;
    std::unique_ptr<Impl> impl_;
};

} // namespace aribtlv
