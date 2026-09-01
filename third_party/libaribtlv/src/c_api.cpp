#include <aribtlv/aribtlv.h>

#include <aribtlv/demuxer.hpp>
#include <aribtlv/duration_probe.hpp>
#include <aribtlv/hlg_sdr_tone_mapping.hpp>
#include <aribtlv/recording.hpp>

#include <algorithm>
#include <cstring>
#include <exception>
#include <new>
#include <optional>
#include <string>
#include <utility>
#include <vector>

namespace {

aribtlv_codec codec(const aribtlv::Codec value) noexcept {
    switch (value) {
    case aribtlv::Codec::Hevc: return ARIBTLV_CODEC_HEVC;
    case aribtlv::Codec::AacLatm: return ARIBTLV_CODEC_AAC_LATM;
    case aribtlv::Codec::Ttml: return ARIBTLV_CODEC_TTML;
    }
    return ARIBTLV_CODEC_HEVC;
}

aribtlv_track_kind track_kind(const aribtlv::TrackKind value) noexcept {
    switch (value) {
    case aribtlv::TrackKind::Video: return ARIBTLV_TRACK_VIDEO;
    case aribtlv::TrackKind::Audio: return ARIBTLV_TRACK_AUDIO;
    case aribtlv::TrackKind::Subtitle: return ARIBTLV_TRACK_SUBTITLE;
    }
    return ARIBTLV_TRACK_VIDEO;
}

std::optional<aribtlv::TrackKind> track_kind(const aribtlv_track_kind value) noexcept {
    switch (value) {
    case ARIBTLV_TRACK_VIDEO: return aribtlv::TrackKind::Video;
    case ARIBTLV_TRACK_AUDIO: return aribtlv::TrackKind::Audio;
    case ARIBTLV_TRACK_SUBTITLE: return aribtlv::TrackKind::Subtitle;
    }
    return std::nullopt;
}

aribtlv_error_code error_code(const aribtlv::ErrorCode value) noexcept {
    switch (value) {
    case aribtlv::ErrorCode::MalformedInput: return ARIBTLV_ERROR_MALFORMED_INPUT;
    case aribtlv::ErrorCode::UnsupportedFeature: return ARIBTLV_ERROR_UNSUPPORTED_FEATURE;
    case aribtlv::ErrorCode::Discontinuity: return ARIBTLV_ERROR_DISCONTINUITY;
    case aribtlv::ErrorCode::ResourceLimit: return ARIBTLV_ERROR_RESOURCE_LIMIT;
    }
    return ARIBTLV_ERROR_MALFORMED_INPUT;
}

std::uint32_t discontinuity_reasons(
    const aribtlv::DiscontinuityReason value) noexcept {
    return static_cast<std::uint32_t>(value);
}

aribtlv_timestamp timestamp(const aribtlv::Timestamp value) noexcept {
    return {value.value, value.timescale};
}

aribtlv_duration_probe_state duration_probe_state(
    const aribtlv::DurationProbeState value) noexcept {
    switch (value) {
    case aribtlv::DurationProbeState::Idle: return ARIBTLV_DURATION_PROBE_IDLE;
    case aribtlv::DurationProbeState::NeedRange: return ARIBTLV_DURATION_PROBE_NEED_RANGE;
    case aribtlv::DurationProbeState::Complete: return ARIBTLV_DURATION_PROBE_COMPLETE;
    case aribtlv::DurationProbeState::Unknown: return ARIBTLV_DURATION_PROBE_UNKNOWN;
    case aribtlv::DurationProbeState::Failed: return ARIBTLV_DURATION_PROBE_FAILED;
    case aribtlv::DurationProbeState::Cancelled: return ARIBTLV_DURATION_PROBE_CANCELLED;
    }
    return ARIBTLV_DURATION_PROBE_FAILED;
}

aribtlv_duration_probe_failure duration_probe_failure(
    const aribtlv::DurationProbeFailure value) noexcept {
    switch (value) {
    case aribtlv::DurationProbeFailure::None:
        return ARIBTLV_DURATION_PROBE_FAILURE_NONE;
    case aribtlv::DurationProbeFailure::InvalidSource:
        return ARIBTLV_DURATION_PROBE_FAILURE_INVALID_SOURCE;
    case aribtlv::DurationProbeFailure::InvalidResponse:
        return ARIBTLV_DURATION_PROBE_FAILURE_INVALID_RESPONSE;
    case aribtlv::DurationProbeFailure::SourceError:
        return ARIBTLV_DURATION_PROBE_FAILURE_SOURCE_ERROR;
    case aribtlv::DurationProbeFailure::NoVideo:
        return ARIBTLV_DURATION_PROBE_FAILURE_NO_VIDEO;
    case aribtlv::DurationProbeFailure::NoTailTimestamp:
        return ARIBTLV_DURATION_PROBE_FAILURE_NO_TAIL_TIMESTAMP;
    case aribtlv::DurationProbeFailure::RangeLimit:
        return ARIBTLV_DURATION_PROBE_FAILURE_RANGE_LIMIT;
    case aribtlv::DurationProbeFailure::ParseError:
        return ARIBTLV_DURATION_PROBE_FAILURE_PARSE_ERROR;
    }
    return ARIBTLV_DURATION_PROBE_FAILURE_PARSE_ERROR;
}

aribtlv_duration_status duration_status(const aribtlv::DurationStatus value) noexcept {
    switch (value) {
    case aribtlv::DurationStatus::Unknown: return ARIBTLV_DURATION_UNKNOWN;
    case aribtlv::DurationStatus::Provisional: return ARIBTLV_DURATION_PROVISIONAL;
    case aribtlv::DurationStatus::Complete: return ARIBTLV_DURATION_COMPLETE;
    }
    return ARIBTLV_DURATION_UNKNOWN;
}

aribtlv_track_info track_info(
    const aribtlv::TrackInfo& source,
    const std::vector<aribtlv_asset_group_info>& asset_groups,
    const aribtlv_subtitle_info* subtitle) noexcept {
    aribtlv_track_info result{};
    result.track_id = source.track_id;
    result.context_id = source.context_id;
    result.packet_id = source.packet_id;
    result.component_tag = source.component_tag;
    result.kind = track_kind(source.kind);
    result.codec = codec(source.codec);
    result.timescale = source.timescale;
    result.language = source.language.c_str();
    if (source.audio) {
        result.has_audio = 1;
        result.audio_main_component = source.audio->main_component ? 1 : 0;
        result.audio_sample_rate = source.audio->sample_rate;
        result.audio_channels = aribtlv::audio_channel_count(source.audio->channel_layout);
    }
    if (source.video) {
        result.has_video = 1;
        if (source.video->hdr_wcg_idc) {
            result.video_has_hdr_wcg_idc = 1;
            result.video_hdr_wcg_idc = *source.video->hdr_wcg_idc;
        }
        if (source.video->video_transfer_characteristics) {
            result.video_has_transfer_characteristics = 1;
            result.video_transfer_characteristics =
                *source.video->video_transfer_characteristics;
        }
    }
    result.asset_groups = asset_groups.empty() ? nullptr : asset_groups.data();
    result.asset_group_count = asset_groups.size();
    result.subtitle = subtitle;
    return result;
}

aribtlv_subtitle_info subtitle_info(const aribtlv::SubtitleInfo& source) noexcept {
    aribtlv_subtitle_info result{};
    result.tag = source.tag;
    result.info_version = source.info_version;
    result.type = source.type;
    result.format = source.format;
    result.operation_mode = source.operation_mode;
    result.timing_mode = source.timing_mode;
    result.display_mode = source.display_mode;
    result.resolution = source.resolution;
    result.compression_type = source.compression_type;
    if (source.start_mpu_sequence_number) {
        result.has_start_mpu_sequence_number = 1;
        result.start_mpu_sequence_number = *source.start_mpu_sequence_number;
    }
    if (source.reference_start_ntp) {
        result.has_reference_start_ntp = 1;
        result.reference_start_ntp = *source.reference_start_ntp;
    }
    result.reference_start_time_leap_indicator = source.reference_start_time_leap_indicator;
    return result;
}

aribtlv_recording_scan_failure recording_scan_failure(
    const aribtlv::RecordingScanFailure value) noexcept {
    switch (value) {
    case aribtlv::RecordingScanFailure::None:
        return ARIBTLV_RECORDING_SCAN_FAILURE_NONE;
    case aribtlv::RecordingScanFailure::SourceError:
        return ARIBTLV_RECORDING_SCAN_FAILURE_SOURCE_ERROR;
    case aribtlv::RecordingScanFailure::NoVideo:
        return ARIBTLV_RECORDING_SCAN_FAILURE_NO_VIDEO;
    case aribtlv::RecordingScanFailure::NoRandomAccessPoint:
        return ARIBTLV_RECORDING_SCAN_FAILURE_NO_RANDOM_ACCESS_POINT;
    case aribtlv::RecordingScanFailure::ParseError:
        return ARIBTLV_RECORDING_SCAN_FAILURE_PARSE_ERROR;
    }
    return ARIBTLV_RECORDING_SCAN_FAILURE_PARSE_ERROR;
}

aribtlv_seek_point seek_point(const aribtlv::SeekPoint& source) noexcept {
    return {
        timestamp(source.presentation_time),
        source.signalling_offset,
        source.random_access_offset,
        source.video_track_id,
        source.bootstrap_id,
    };
}

class CallbackSink final : public aribtlv::Sink {
public:
    CallbackSink(aribtlv_callbacks callbacks, void* opaque)
        : callbacks_(callbacks), opaque_(opaque) {}

    void beginCall() {
        fatal_error_ = false;
        last_error_.clear();
    }

    bool fatalError() const noexcept { return fatal_error_; }
    const std::string& lastError() const noexcept { return last_error_; }
    void setLastError(std::string message) { last_error_ = std::move(message); }

    void onService(const aribtlv::ServiceInfo& source) override {
        if (!callbacks_.on_service) return;
        const aribtlv_service_info event{
            source.context_id,
            source.package_id.empty() ? nullptr : source.package_id.data(),
            source.package_id.size(),
        };
        callbacks_.on_service(opaque_, &event);
    }

    void onTrack(const aribtlv::TrackInfo& source) override {
        if (!callbacks_.on_track) return;
        convertAssetGroups(source);
        convertSubtitleInfo(source);
        const auto event = track_info(source, asset_groups_, subtitle_ ? &*subtitle_ : nullptr);
        callbacks_.on_track(opaque_, &event);
    }

    void onTrackRemoved(const aribtlv::TrackInfo& source) override {
        if (!callbacks_.on_track_removed) return;
        convertAssetGroups(source);
        convertSubtitleInfo(source);
        const auto event = track_info(source, asset_groups_, subtitle_ ? &*subtitle_ : nullptr);
        callbacks_.on_track_removed(opaque_, &event);
    }

    void onAccessUnit(aribtlv::AccessUnit&& source) override {
        if (!callbacks_.on_access_unit) return;
        subtitle_resources_.clear();
        subtitle_resources_.reserve(source.subtitle_resources.size());
        for (const auto& resource : source.subtitle_resources) {
            subtitle_resources_.push_back({
                resource.subsample_number,
                resource.data_type,
                resource.data.empty() ? nullptr : resource.data.data(),
                resource.data.size(),
            });
        }
        aribtlv_access_unit event{};
        event.track_id = source.track_id;
        event.codec = codec(source.codec);
        event.component_tag = source.component_tag;
        event.data = source.data.empty() ? nullptr : source.data.data();
        event.size = source.data.size();
        event.pts = timestamp(source.pts);
        event.dts = timestamp(source.dts);
        event.restart_offset = source.restart_offset;
        event.input_offset = source.input_offset;
        event.random_access = static_cast<std::uint8_t>(source.random_access ? 1 : 0);
        event.discontinuity = static_cast<std::uint8_t>(source.discontinuity ? 1 : 0);
        if (source.subtitle_timing_mode) {
            event.has_subtitle_timing_mode = 1;
            event.subtitle_timing_mode = *source.subtitle_timing_mode;
        }
        if (source.subtitle_operation_mode) {
            event.has_subtitle_operation_mode = 1;
            event.subtitle_operation_mode = *source.subtitle_operation_mode;
        }
        if (source.subtitle_display_mode) {
            event.has_subtitle_display_mode = 1;
            event.subtitle_display_mode = *source.subtitle_display_mode;
        }
        if (source.subtitle_compression_type) {
            event.has_subtitle_compression_type = 1;
            event.subtitle_compression_type = *source.subtitle_compression_type;
        }
        if (source.mpu_sequence_number) {
            event.has_mpu_sequence_number = 1;
            event.mpu_sequence_number = *source.mpu_sequence_number;
        }
        if (source.subtitle_reference_start_pts) {
            event.has_subtitle_reference_start_pts = 1;
            event.subtitle_reference_start_pts = timestamp(*source.subtitle_reference_start_pts);
        }
        event.subtitle_resources = subtitle_resources_.empty() ? nullptr : subtitle_resources_.data();
        event.subtitle_resource_count = subtitle_resources_.size();
        event.discontinuity_reasons = discontinuity_reasons(source.discontinuity_reasons);
        callbacks_.on_access_unit(opaque_, &event);
    }

    void onDamage(const aribtlv::DamageSpan& source) override {
        if (!callbacks_.on_damage) return;
        aribtlv_damage_span event{};
        event.track_id = source.track_id;
        event.kind = track_kind(source.kind);
        event.codec = codec(source.codec);
        if (source.start_time) {
            event.has_start_time = 1;
            event.start_time = timestamp(*source.start_time);
        }
        event.end_time = timestamp(source.end_time);
        if (source.recovery_time) {
            event.has_recovery_time = 1;
            event.recovery_time = timestamp(*source.recovery_time);
        }
        event.start_input_offset = source.start_input_offset;
        event.end_input_offset = source.end_input_offset;
        event.recovery_input_offset = source.recovery_input_offset;
        event.recovery_restart_offset = source.recovery_restart_offset;
        event.reasons = discontinuity_reasons(source.reasons);
        event.recovered = static_cast<std::uint8_t>(source.recovered ? 1 : 0);
        event.recovery_random_access =
            static_cast<std::uint8_t>(source.recovery_random_access ? 1 : 0);
        callbacks_.on_damage(opaque_, &event);
    }

    void onIpDataFlow(const aribtlv::IpDataFlow& source) override {
        if (!callbacks_.on_ip_data_flow) return;
        aribtlv_ip_data_flow event{};
        event.context_id = source.context_id;
        event.sequence_number = source.sequence_number;
        event.ip_version = source.ip_version;
        std::copy(source.source_address.begin(), source.source_address.end(),
                  event.source_address);
        std::copy(source.destination_address.begin(), source.destination_address.end(),
                  event.destination_address);
        event.next_header = source.next_header;
        event.source_port = source.source_port;
        event.destination_port = source.destination_port;
        event.input_offset = source.input_offset;
        callbacks_.on_ip_data_flow(opaque_, &event);
    }

    void onTransportNtpClock(const aribtlv::TransportNtpClock& source) override {
        if (!callbacks_.on_transport_ntp_clock) return;
        aribtlv_transport_ntp_clock event{};
        event.ip_version = source.ip_version;
        std::copy(source.source_address.begin(), source.source_address.end(),
                  event.source_address);
        std::copy(source.destination_address.begin(), source.destination_address.end(),
                  event.destination_address);
        event.source_port = source.source_port;
        event.destination_port = source.destination_port;
        event.leap_indicator = source.leap_indicator;
        event.version = source.version;
        event.mode = source.mode;
        event.stratum = source.stratum;
        event.poll = source.poll;
        event.precision = source.precision;
        event.root_delay = source.root_delay;
        event.root_dispersion = source.root_dispersion;
        event.reference_identification = source.reference_identification;
        event.reference_timestamp = source.reference_timestamp;
        event.origin_timestamp = source.origin_timestamp;
        event.receive_timestamp = source.receive_timestamp;
        event.transmit_timestamp = source.transmit_timestamp;
        event.transmit_time = timestamp(source.transmit_time);
        event.input_offset = source.input_offset;
        callbacks_.on_transport_ntp_clock(opaque_, &event);
    }

    void onTlvNetworkInformation(
        const aribtlv::TlvNetworkInformation& source) override {
        if (!callbacks_.on_tlv_network_information) return;
        std::size_t descriptor_count = source.network_descriptors.size();
        for (const auto& stream : source.streams) {
            descriptor_count += stream.descriptors.size();
        }
        tlv_descriptors_.clear();
        tlv_descriptors_.reserve(descriptor_count);
        for (const auto& descriptor : source.network_descriptors) {
            tlv_descriptors_.push_back(descriptorView(descriptor));
        }
        std::vector<std::pair<std::size_t, std::size_t>> stream_ranges;
        stream_ranges.reserve(source.streams.size());
        for (const auto& stream : source.streams) {
            const auto begin = tlv_descriptors_.size();
            for (const auto& descriptor : stream.descriptors) {
                tlv_descriptors_.push_back(descriptorView(descriptor));
            }
            stream_ranges.emplace_back(begin, stream.descriptors.size());
        }
        tlv_streams_.clear();
        tlv_streams_.reserve(source.streams.size());
        for (std::size_t index = 0; index < source.streams.size(); ++index) {
            const auto& stream = source.streams[index];
            const auto [begin, count] = stream_ranges[index];
            tlv_streams_.push_back({
                stream.tlv_stream_id,
                stream.original_network_id,
                count == 0 ? nullptr : tlv_descriptors_.data() + begin,
                count,
            });
        }
        const aribtlv_tlv_network_information event{
            source.table_id,
            source.network_id,
            source.version,
            static_cast<std::uint8_t>(source.current_next ? 1 : 0),
            source.last_section_number,
            source.network_descriptors.empty() ? nullptr : tlv_descriptors_.data(),
            source.network_descriptors.size(),
            tlv_streams_.empty() ? nullptr : tlv_streams_.data(),
            tlv_streams_.size(),
            source.input_offset,
        };
        callbacks_.on_tlv_network_information(opaque_, &event);
    }

    void onAddressMap(const aribtlv::AddressMap& source) override {
        if (!callbacks_.on_address_map) return;
        address_map_services_.clear();
        address_map_services_.reserve(source.services.size());
        for (const auto& service : source.services) {
            aribtlv_address_map_service converted{};
            converted.service_id = service.service_id;
            converted.ip_version = service.ip_version;
            std::copy(service.source_address.begin(), service.source_address.end(),
                      converted.source_address);
            converted.source_prefix_length = service.source_prefix_length;
            std::copy(service.destination_address.begin(),
                      service.destination_address.end(), converted.destination_address);
            converted.destination_prefix_length = service.destination_prefix_length;
            converted.private_data = service.private_data.empty()
                ? nullptr : service.private_data.data();
            converted.private_data_size = service.private_data.size();
            address_map_services_.push_back(converted);
        }
        const aribtlv_address_map event{
            source.table_id,
            source.table_id_extension,
            source.version,
            static_cast<std::uint8_t>(source.current_next ? 1 : 0),
            source.last_section_number,
            address_map_services_.empty() ? nullptr : address_map_services_.data(),
            address_map_services_.size(),
            source.input_offset,
        };
        callbacks_.on_address_map(opaque_, &event);
    }

    void onRawSignallingTable(aribtlv::RawSignallingTable&& source) override {
        if (!callbacks_.on_raw_signalling_table) return;
        const aribtlv_raw_signalling_table event{
            source.tlv_packet_type,
            source.table_id,
            source.table_id_extension,
            source.version,
            static_cast<std::uint8_t>(source.current_next ? 1 : 0),
            source.section_number,
            source.last_section_number,
            source.data.empty() ? nullptr : source.data.data(),
            source.data.size(),
            source.input_offset,
        };
        callbacks_.on_raw_signalling_table(opaque_, &event);
    }

    void onUnknownDescriptor(aribtlv::UnknownDescriptor&& source) override {
        if (!callbacks_.on_unknown_descriptor) return;
        aribtlv_unknown_descriptor event{};
        event.table_id = source.table_id;
        event.tag = source.tag;
        event.scope = source.scope == aribtlv::DescriptorScope::TlvStream
            ? ARIBTLV_DESCRIPTOR_TLV_STREAM : ARIBTLV_DESCRIPTOR_NETWORK;
        if (source.tlv_stream_id) {
            event.has_tlv_stream_id = 1;
            event.tlv_stream_id = *source.tlv_stream_id;
        }
        if (source.original_network_id) {
            event.has_original_network_id = 1;
            event.original_network_id = *source.original_network_id;
        }
        event.section_offset = source.section_offset;
        event.payload = source.payload.empty() ? nullptr : source.payload.data();
        event.payload_size = source.payload.size();
        event.input_offset = source.input_offset;
        callbacks_.on_unknown_descriptor(opaque_, &event);
    }

    void onError(const aribtlv::Error& source) override {
        if (!source.recoverable) {
            fatal_error_ = true;
            last_error_ = source.message;
        }
        if (!callbacks_.on_error) return;
        const aribtlv_error event{
            error_code(source.code),
            source.input_offset,
            static_cast<std::uint8_t>(source.recoverable ? 1 : 0),
            source.message.c_str(),
        };
        callbacks_.on_error(opaque_, &event);
    }

private:
    static aribtlv_tlv_descriptor descriptorView(
        const aribtlv::TlvDescriptor& source) noexcept {
        return {
            source.tag,
            source.payload.empty() ? nullptr : source.payload.data(),
            source.payload.size(),
            source.section_offset,
        };
    }

    void convertAssetGroups(const aribtlv::TrackInfo& source) {
        asset_groups_.clear();
        asset_groups_.reserve(source.asset_groups.size());
        for (const auto& group : source.asset_groups) {
            asset_groups_.push_back({group.group_identification, group.selection_level});
        }
    }

    void convertSubtitleInfo(const aribtlv::TrackInfo& source) {
        subtitle_.reset();
        if (source.subtitle) subtitle_ = subtitle_info(*source.subtitle);
    }

    aribtlv_callbacks callbacks_{};
    void* opaque_ = nullptr;
    bool fatal_error_ = false;
    std::string last_error_;
    std::vector<aribtlv_asset_group_info> asset_groups_;
    std::optional<aribtlv_subtitle_info> subtitle_;
    std::vector<aribtlv_subtitle_resource> subtitle_resources_;
    std::vector<aribtlv_tlv_descriptor> tlv_descriptors_;
    std::vector<aribtlv_tlv_network_stream> tlv_streams_;
    std::vector<aribtlv_address_map_service> address_map_services_;
};

} // namespace

struct aribtlv_demuxer {
    aribtlv_demuxer(aribtlv_callbacks callbacks, void* opaque, aribtlv::Limits limits)
        : sink(callbacks, opaque), implementation(sink, limits) {}

    CallbackSink sink;
    aribtlv::Demuxer implementation;
};

struct aribtlv_duration_probe {
    aribtlv::DurationProbe implementation;
};

struct aribtlv_recording_scanner {
    explicit aribtlv_recording_scanner(aribtlv::RecordingScanOptions options)
        : implementation(std::move(options)) {}

    void cacheResult(const aribtlv::RecordingScanResult& source) {
        if (cached) return;
        seek_points.reserve(source.seek_points.size());
        for (const auto& point : source.seek_points) seek_points.push_back(seek_point(point));
        if (source.error) error_message = source.error->message;

        result.failure = recording_scan_failure(source.failure);
        if (source.error) {
            result.has_error = 1;
            result.error = {
                error_code(source.error->code),
                source.error->input_offset,
                static_cast<std::uint8_t>(source.error->recoverable ? 1 : 0),
                error_message.c_str(),
            };
        }
        if (source.video_track_id) {
            result.has_video_track = 1;
            result.video_track_id = *source.video_track_id;
        }
        if (source.video_packet_id) {
            result.has_video_packet_id = 1;
            result.video_packet_id = *source.video_packet_id;
        }
        if (source.first_presentation_time) {
            result.has_first_presentation_time = 1;
            result.first_presentation_time = timestamp(*source.first_presentation_time);
        }
        if (source.last_presentation_time) {
            result.has_last_presentation_time = 1;
            result.last_presentation_time = timestamp(*source.last_presentation_time);
        }
        result.duration = {timestamp(source.duration.value), duration_status(source.duration.status)};
        result.seek_points = seek_points.empty() ? nullptr : seek_points.data();
        result.seek_point_count = seek_points.size();
        cached = true;
    }

    aribtlv::RecordingScanner implementation;
    aribtlv_recording_scan_result result{};
    std::vector<aribtlv_seek_point> seek_points;
    std::string error_message;
    bool cached = false;
};

namespace {

template <typename Operation>
int invoke(aribtlv_demuxer* demuxer, Operation operation) noexcept {
    if (!demuxer) return ARIBTLV_ERROR_INVALID_ARGUMENT;
    demuxer->sink.beginCall();
    try {
        operation(demuxer->implementation);
        return demuxer->sink.fatalError() ? ARIBTLV_ERROR_DEMUX : ARIBTLV_OK;
    } catch (const std::bad_alloc&) {
        demuxer->sink.setLastError("out of memory");
        return ARIBTLV_ERROR_OUT_OF_MEMORY;
    } catch (const std::exception& error) {
        demuxer->sink.setLastError(error.what());
        return ARIBTLV_ERROR_INTERNAL;
    } catch (...) {
        demuxer->sink.setLastError("unknown C++ exception");
        return ARIBTLV_ERROR_INTERNAL;
    }
}

} // namespace

extern "C" {

uint32_t aribtlv_version(void) { return ARIBTLV_VERSION_INT; }

const char* aribtlv_version_string(void) { return ARIBTLV_VERSION_STRING; }

int aribtlv_hlg_sdr_lut_describe(const aribtlv_hlg_sdr_lut_profile profile,
                                 aribtlv_hlg_sdr_lut_info* info) {
    if (!info) return ARIBTLV_ERROR_INVALID_ARGUMENT;
    size_t dimension = 0;
    switch (profile) {
    case ARIBTLV_HLG_SDR_LUT_DISPLAY:
        dimension = aribtlv::kHlgSdrColorLutSize;
        break;
    case ARIBTLV_HLG_SDR_LUT_BT2446_PROTOTYPE:
        dimension = aribtlv::kHlgSdrPrototypeColorLutSize;
        break;
    default:
        return ARIBTLV_ERROR_INVALID_ARGUMENT;
    }
    info->dimension = static_cast<uint32_t>(dimension);
    info->rgb_float_count = dimension * dimension * dimension * 3U;
    return ARIBTLV_OK;
}

int aribtlv_hlg_sdr_lut_generate(const aribtlv_hlg_sdr_lut_profile profile,
                                 float* rgb, const size_t rgb_float_count) {
    aribtlv_hlg_sdr_lut_info info{};
    const auto described = aribtlv_hlg_sdr_lut_describe(profile, &info);
    if (described != ARIBTLV_OK || !rgb) return ARIBTLV_ERROR_INVALID_ARGUMENT;
    if (rgb_float_count < info.rgb_float_count) return ARIBTLV_ERROR_BUFFER_TOO_SMALL;
    try {
        const auto lut = profile == ARIBTLV_HLG_SDR_LUT_BT2446_PROTOTYPE
            ? aribtlv::hlg_sdr_prototype_color_lut()
            : aribtlv::hlg_sdr_color_lut();
        const auto columns = lut.width / lut.size;
        size_t output = 0;
        for (size_t blue = 0; blue < lut.size; ++blue) {
            for (size_t green = 0; green < lut.size; ++green) {
                for (size_t red = 0; red < lut.size; ++red) {
                    const auto x = (blue % columns) * lut.size + red;
                    const auto y = (blue / columns) * lut.size + green;
                    const auto offset = (y * lut.width + x) * 4U;
                    rgb[output++] = static_cast<float>(lut.rgba[offset]) / 255.0F;
                    rgb[output++] = static_cast<float>(lut.rgba[offset + 1U]) / 255.0F;
                    rgb[output++] = static_cast<float>(lut.rgba[offset + 2U]) / 255.0F;
                }
            }
        }
        return ARIBTLV_OK;
    } catch (const std::bad_alloc&) {
        return ARIBTLV_ERROR_OUT_OF_MEMORY;
    } catch (...) {
        return ARIBTLV_ERROR_INTERNAL;
    }
}

void aribtlv_callbacks_init(aribtlv_callbacks* callbacks) {
    if (!callbacks) return;
    std::memset(callbacks, 0, sizeof(*callbacks));
    callbacks->struct_size = sizeof(*callbacks);
}

void aribtlv_config_init(aribtlv_config* config) {
    if (!config) return;
    std::memset(config, 0, sizeof(*config));
    config->struct_size = sizeof(*config);
    config->collect_application_resources = 1;
}

void aribtlv_duration_probe_options_init(aribtlv_duration_probe_options* options) {
    if (!options) return;
    std::memset(options, 0, sizeof(*options));
    options->struct_size = sizeof(*options);
    options->initial_range_size = 4ULL * 1024ULL * 1024ULL;
    options->max_range_size = 64ULL * 1024ULL * 1024ULL;
}

void aribtlv_recording_scan_options_init(aribtlv_recording_scan_options* options) {
    if (!options) return;
    std::memset(options, 0, sizeof(*options));
    options->struct_size = sizeof(*options);
}

aribtlv_demuxer* aribtlv_demuxer_create(
    const aribtlv_config* config, const aribtlv_callbacks* callbacks, void* opaque) {
    try {
        aribtlv_callbacks copied_callbacks{};
        if (callbacks) {
            if (callbacks->struct_size < sizeof(callbacks->struct_size)) return nullptr;
            std::memcpy(&copied_callbacks, callbacks,
                        std::min(callbacks->struct_size, sizeof(copied_callbacks)));
        }
        aribtlv::Limits limits;
        if (config) {
            const auto required = offsetof(aribtlv_config, collect_application_resources) +
                sizeof(config->collect_application_resources);
            if (config->struct_size < required) return nullptr;
            limits.collect_application_resources = config->collect_application_resources != 0;
        }
        return new aribtlv_demuxer(copied_callbacks, opaque, limits);
    } catch (...) {
        return nullptr;
    }
}

void aribtlv_demuxer_destroy(aribtlv_demuxer* demuxer) { delete demuxer; }

int aribtlv_demuxer_push(aribtlv_demuxer* demuxer, const uint8_t* data, const size_t size) {
    if (!data && size != 0) return ARIBTLV_ERROR_INVALID_ARGUMENT;
    return invoke(demuxer, [&](aribtlv::Demuxer& value) { value.push(data, size); });
}

int aribtlv_demuxer_flush(aribtlv_demuxer* demuxer) {
    return invoke(demuxer, [](aribtlv::Demuxer& value) { value.flush(); });
}

int aribtlv_demuxer_reset(aribtlv_demuxer* demuxer) {
    return invoke(demuxer, [](aribtlv::Demuxer& value) { value.reset(); });
}

int aribtlv_demuxer_reposition(aribtlv_demuxer* demuxer, const uint64_t input_offset,
                               const uint8_t preserve_timeline) {
    return invoke(demuxer, [&](aribtlv::Demuxer& value) {
        value.reposition({input_offset, preserve_timeline != 0});
    });
}

int aribtlv_demuxer_select_service(aribtlv_demuxer* demuxer, const uint32_t context_id) {
    return invoke(demuxer, [&](aribtlv::Demuxer& value) { value.selectService(context_id); });
}

int aribtlv_demuxer_clear_service(aribtlv_demuxer* demuxer) {
    return invoke(demuxer, [](aribtlv::Demuxer& value) { value.selectService(std::nullopt); });
}

int aribtlv_demuxer_select_track(aribtlv_demuxer* demuxer, const aribtlv_track_kind kind,
                                 const uint64_t track_id) {
    const auto converted = track_kind(kind);
    if (!converted) return ARIBTLV_ERROR_INVALID_ARGUMENT;
    return invoke(demuxer, [&](aribtlv::Demuxer& value) {
        value.selectTrack(*converted, track_id);
    });
}

int aribtlv_demuxer_clear_track(aribtlv_demuxer* demuxer, const aribtlv_track_kind kind) {
    const auto converted = track_kind(kind);
    if (!converted) return ARIBTLV_ERROR_INVALID_ARGUMENT;
    return invoke(demuxer, [&](aribtlv::Demuxer& value) {
        value.selectTrack(*converted, std::nullopt);
    });
}

int aribtlv_demuxer_set_subtitle_passthrough(aribtlv_demuxer* demuxer,
                                             const uint8_t enabled) {
    return invoke(demuxer, [&](aribtlv::Demuxer& value) {
        value.setSubtitlePassthroughEnabled(enabled != 0);
    });
}

const char* aribtlv_demuxer_last_error(const aribtlv_demuxer* demuxer) {
    return demuxer ? demuxer->sink.lastError().c_str() : "invalid demuxer";
}

aribtlv_duration_probe* aribtlv_duration_probe_create(void) {
    try {
        return new aribtlv_duration_probe;
    } catch (...) {
        return nullptr;
    }
}

void aribtlv_duration_probe_destroy(aribtlv_duration_probe* probe) { delete probe; }

int aribtlv_duration_probe_begin(aribtlv_duration_probe* probe, const uint64_t source_size,
                                 const aribtlv_duration_probe_options* options) {
    if (!probe) return ARIBTLV_ERROR_INVALID_ARGUMENT;
    try {
        aribtlv::DurationProbeOptions converted;
        if (options) {
            const auto required = offsetof(aribtlv_duration_probe_options, max_range_size) +
                sizeof(options->max_range_size);
            if (options->struct_size < required) return ARIBTLV_ERROR_INVALID_ARGUMENT;
            converted.initial_range_size = options->initial_range_size;
            converted.max_range_size = options->max_range_size;
            const auto service_end = offsetof(aribtlv_duration_probe_options,
                                              service_context_id) +
                sizeof(options->service_context_id);
            if (options->struct_size >= service_end && options->has_service_context_id)
                converted.service_context_id = options->service_context_id;
            const auto packet_end = offsetof(aribtlv_duration_probe_options,
                                             video_packet_id) +
                sizeof(options->video_packet_id);
            if (options->struct_size >= packet_end && options->has_video_packet_id)
                converted.video_packet_id = options->video_packet_id;
        }
        probe->implementation.begin(source_size, converted);
        return ARIBTLV_OK;
    } catch (const std::bad_alloc&) {
        return ARIBTLV_ERROR_OUT_OF_MEMORY;
    } catch (...) {
        return ARIBTLV_ERROR_INTERNAL;
    }
}

int aribtlv_duration_probe_next_range(const aribtlv_duration_probe* probe,
                                      aribtlv_range_request* request) {
    if (!probe || !request) return ARIBTLV_ERROR_INVALID_ARGUMENT;
    const auto value = probe->implementation.nextRange();
    if (!value) return 0;
    *request = {value->generation, value->request_id, value->offset, value->length};
    return 1;
}

int aribtlv_duration_probe_push_range(aribtlv_duration_probe* probe,
                                      const uint64_t request_id,
                                      const uint64_t absolute_offset,
                                      const uint8_t* data, const size_t size,
                                      const uint8_t end_of_range) {
    if (!probe || (!data && size != 0)) return ARIBTLV_ERROR_INVALID_ARGUMENT;
    try {
        return probe->implementation.pushRange(request_id, absolute_offset, data, size,
                                               end_of_range != 0)
            ? ARIBTLV_OK : ARIBTLV_ERROR_INVALID_ARGUMENT;
    } catch (const std::bad_alloc&) {
        return ARIBTLV_ERROR_OUT_OF_MEMORY;
    } catch (...) {
        return ARIBTLV_ERROR_INTERNAL;
    }
}

int aribtlv_duration_probe_fail_range(aribtlv_duration_probe* probe,
                                      const uint64_t request_id) {
    if (!probe) return ARIBTLV_ERROR_INVALID_ARGUMENT;
    return probe->implementation.failRange(request_id)
        ? ARIBTLV_OK : ARIBTLV_ERROR_INVALID_ARGUMENT;
}

void aribtlv_duration_probe_cancel(aribtlv_duration_probe* probe) {
    if (probe) probe->implementation.cancel();
}

aribtlv_duration_probe_state aribtlv_duration_probe_get_state(
    const aribtlv_duration_probe* probe) {
    return probe ? duration_probe_state(probe->implementation.state())
                 : ARIBTLV_DURATION_PROBE_FAILED;
}

aribtlv_duration_probe_failure aribtlv_duration_probe_get_failure(
    const aribtlv_duration_probe* probe) {
    return probe ? duration_probe_failure(probe->implementation.failure())
                 : ARIBTLV_DURATION_PROBE_FAILURE_INVALID_SOURCE;
}

int aribtlv_duration_probe_get_duration(const aribtlv_duration_probe* probe,
                                        aribtlv_duration_info* duration) {
    if (!probe || !duration) return ARIBTLV_ERROR_INVALID_ARGUMENT;
    const auto value = probe->implementation.duration();
    duration->value = timestamp(value.value);
    duration->status = duration_status(value.status);
    return ARIBTLV_OK;
}

int aribtlv_duration_probe_get_presentation_start(
    const aribtlv_duration_probe* probe, aribtlv_timestamp* presentation_start) {
    if (!probe || !presentation_start) return ARIBTLV_ERROR_INVALID_ARGUMENT;
    const auto value = probe->implementation.presentationStart();
    if (!value.has_value()) return ARIBTLV_ERROR_DEMUX;
    *presentation_start = timestamp(*value);
    return ARIBTLV_OK;
}

int aribtlv_duration_probe_get_presentation_end(
    const aribtlv_duration_probe* probe, aribtlv_timestamp* presentation_end) {
    if (!probe || !presentation_end) return ARIBTLV_ERROR_INVALID_ARGUMENT;
    const auto value = probe->implementation.presentationEnd();
    if (!value.has_value()) return ARIBTLV_ERROR_DEMUX;
    *presentation_end = timestamp(*value);
    return ARIBTLV_OK;
}

int aribtlv_duration_probe_get_presentation_end_video_packet_id(
    const aribtlv_duration_probe* probe, uint16_t* video_packet_id) {
    if (!probe || !video_packet_id) return ARIBTLV_ERROR_INVALID_ARGUMENT;
    const auto value = probe->implementation.presentationEndVideoPacketId();
    if (!value.has_value()) return ARIBTLV_ERROR_DEMUX;
    *video_packet_id = *value;
    return ARIBTLV_OK;
}

uint64_t aribtlv_duration_probe_transferred_bytes(const aribtlv_duration_probe* probe) {
    return probe ? probe->implementation.transferredBytes() : 0;
}

aribtlv_recording_scanner* aribtlv_recording_scanner_create(
    const aribtlv_recording_scan_options* options) {
    try {
        aribtlv::RecordingScanOptions converted;
        if (options) {
            if (options->struct_size < sizeof(options->struct_size)) return nullptr;
            const auto service_end = offsetof(aribtlv_recording_scan_options,
                                              service_context_id) +
                sizeof(options->service_context_id);
            if (options->struct_size >= service_end && options->has_service_context_id) {
                converted.service_context_id = options->service_context_id;
            }
            const auto packet_end = offsetof(aribtlv_recording_scan_options,
                                             video_packet_id) +
                sizeof(options->video_packet_id);
            if (options->struct_size >= packet_end && options->has_video_packet_id) {
                converted.video_packet_id = options->video_packet_id;
            }
        }
        return new aribtlv_recording_scanner(std::move(converted));
    } catch (...) {
        return nullptr;
    }
}

void aribtlv_recording_scanner_destroy(aribtlv_recording_scanner* scanner) {
    delete scanner;
}

int aribtlv_recording_scanner_push(aribtlv_recording_scanner* scanner,
                                   const uint8_t* data, const size_t size) {
    if (!scanner || (!data && size != 0)) return ARIBTLV_ERROR_INVALID_ARGUMENT;
    try {
        return scanner->implementation.push(data, size) ? ARIBTLV_OK : ARIBTLV_ERROR_DEMUX;
    } catch (const std::bad_alloc&) {
        return ARIBTLV_ERROR_OUT_OF_MEMORY;
    } catch (...) {
        return ARIBTLV_ERROR_INTERNAL;
    }
}

void aribtlv_recording_scanner_fail_source(aribtlv_recording_scanner* scanner) {
    if (scanner) scanner->implementation.failSource();
}

int aribtlv_recording_scanner_finish(aribtlv_recording_scanner* scanner,
                                     aribtlv_recording_scan_result* result) {
    if (!scanner || !result) return ARIBTLV_ERROR_INVALID_ARGUMENT;
    try {
        scanner->cacheResult(scanner->implementation.finish());
        *result = scanner->result;
        return ARIBTLV_OK;
    } catch (const std::bad_alloc&) {
        return ARIBTLV_ERROR_OUT_OF_MEMORY;
    } catch (...) {
        return ARIBTLV_ERROR_INTERNAL;
    }
}

int aribtlv_recording_scanner_seek_from_start(
    const aribtlv_recording_scanner* scanner, const aribtlv_timestamp offset,
    aribtlv_recording_seek_result* result) {
    if (!scanner || !result) return ARIBTLV_ERROR_INVALID_ARGUMENT;
    const auto found = scanner->implementation.seekFromStart(
        aribtlv::Timestamp{offset.value, offset.timescale});
    if (!found) return 0;
    result->target_presentation_time = timestamp(found->target_presentation_time);
    result->point = seek_point(found->point);
    return 1;
}

} // extern "C"
