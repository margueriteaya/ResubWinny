#pragma once

#include <cstddef>
#include <cstdint>
#include <memory>
#include <optional>

#include <aribtlv/recording.hpp>

namespace aribtlv {

struct RangeRequest {
    std::uint64_t generation = 0;
    std::uint64_t request_id = 0;
    std::uint64_t offset = 0;
    std::uint64_t length = 0;
};

struct DurationProbeOptions {
    std::uint64_t initial_range_size = 4ULL * 1024ULL * 1024ULL;
    std::uint64_t max_range_size = 64ULL * 1024ULL * 1024ULL;
    std::optional<std::uint32_t> service_context_id;
    std::optional<std::uint16_t> video_packet_id;
};

enum class DurationProbeState {
    Idle,
    NeedRange,
    Complete,
    Unknown,
    Failed,
    Cancelled,
};

enum class DurationProbeFailure {
    None,
    InvalidSource,
    InvalidResponse,
    SourceError,
    NoVideo,
    NoTailTimestamp,
    RangeLimit,
    ParseError,
};

class DurationProbe {
public:
    DurationProbe();
    ~DurationProbe();

    DurationProbe(DurationProbe&&) noexcept;
    DurationProbe& operator=(DurationProbe&&) noexcept;
    DurationProbe(const DurationProbe&) = delete;
    DurationProbe& operator=(const DurationProbe&) = delete;

    bool begin(std::uint64_t source_size, DurationProbeOptions = {});
    std::optional<RangeRequest> nextRange() const noexcept;
    bool pushRange(std::uint64_t request_id, std::uint64_t absolute_offset,
                   const std::uint8_t* data, std::size_t size, bool end_of_range);
    bool failRange(std::uint64_t request_id);
    void cancel() noexcept;

    DurationProbeState state() const noexcept;
    DurationProbeFailure failure() const noexcept;
    DurationInfo duration() const noexcept;
    std::optional<Timestamp> presentationStart() const noexcept;
    std::optional<Timestamp> presentationEnd() const noexcept;
    std::optional<std::uint16_t> selectedVideoPacketId() const noexcept;
    std::optional<std::uint16_t> presentationEndVideoPacketId() const noexcept;
    std::uint64_t generation() const noexcept;
    std::uint64_t transferredBytes() const noexcept;

private:
    class Impl;
    std::unique_ptr<Impl> impl_;
};

} // namespace aribtlv
