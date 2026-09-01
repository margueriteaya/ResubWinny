#include "mmtp_parser.hpp"

#include <algorithm>
#include <limits>
#include <string>
#include <utility>

#include "byte_reader.hpp"

namespace aribtlv::detail {

namespace {

bool parse_packet_extensions(const std::uint16_t extension_type,
                             const std::uint8_t* extension,
                             const std::size_t extension_size,
                             MmtpParser::PacketExtensions& result) {
    if (extension_type != 0x0000) return true;

    std::size_t cursor = 0;
    while (cursor < extension_size) {
        if (extension_size - cursor < 4) return false;
        const auto header = read_be16(extension + cursor);
        const auto type = static_cast<std::uint16_t>(header & 0x7fffU);
        const bool end = (header & 0x8000U) != 0;
        const auto size = static_cast<std::size_t>(read_be16(extension + cursor + 2));
        cursor += 4;
        if (size > extension_size - cursor) return false;

        if (type == 0x0001) {
            if (size < 1) return false;
            const auto flags = extension[cursor];
            const bool scramble_system_present = (flags & 0x04U) != 0;
            const bool authentication_present = (flags & 0x02U) != 0;
            std::size_t field_cursor = 1;
            if (scramble_system_present) {
                if (field_cursor >= size) return false;
                ++field_cursor;
            }
            if (authentication_present) {
                if (size - field_cursor < 2) return false;
                result.authenticated_payload_size = read_be16(extension + cursor + field_cursor);
            }
        } else if (type == 0x0002 && size == 4) {
            result.download_id = read_be32(extension + cursor);
        } else if (type == 0x0003 && size == 8) {
            result.item_fragment_number = read_be32(extension + cursor);
            result.last_item_fragment_number = read_be32(extension + cursor + 4);
        }

        cursor += size;
        if (end) return cursor == extension_size;
    }
    return true;
}

} // namespace

MmtpParser::MmtpParser(const std::uint32_t context_id, const Limits& limits,
                       PackageCallback on_package, TrackCallback on_track,
                       AccessUnitCallback on_access_unit,
                       ApplicationServiceCallback on_application_service,
                       LayoutCallback on_layout,
                       DataAssetCallback on_data_asset,
                       DataUnitCallback on_data_unit,
                       SignallingCallback on_signalling,
                       EventCallback on_event,
                       MhSdtCallback on_mh_sdt,
                       MhTotCallback on_mh_tot,
                       StreamEventCallback on_stream_event,
                       ViewerParticipationCallback on_viewer_participation,
                       ApplicationCallback on_application,
                       MptSnapshotCallback on_mpt_snapshot,
                       MhAitSnapshotCallback on_mh_ait_snapshot,
                       DataTransmissionCallback on_data_transmission,
                       DataDirectoryCallback on_data_directory,
                       DataAssetManagementCallback on_data_asset_management,
                       StateAcquireCallback acquire_state,
                       StateReleaseCallback release_state, ErrorCallback on_error)
    : context_id_(context_id), limits_(limits), on_package_(std::move(on_package)),
      on_track_(std::move(on_track)), on_access_unit_(std::move(on_access_unit)),
      on_application_service_(std::move(on_application_service)),
      on_layout_(std::move(on_layout)),
      on_data_asset_(std::move(on_data_asset)), on_data_unit_(std::move(on_data_unit)),
      on_signalling_(std::move(on_signalling)),
      on_event_(std::move(on_event)),
      on_mh_sdt_(std::move(on_mh_sdt)),
      on_mh_tot_(std::move(on_mh_tot)),
      on_stream_event_(std::move(on_stream_event)),
      on_viewer_participation_(std::move(on_viewer_participation)),
      on_application_(std::move(on_application)),
      on_mpt_snapshot_(std::move(on_mpt_snapshot)),
      on_mh_ait_snapshot_(std::move(on_mh_ait_snapshot)),
      on_data_transmission_(std::move(on_data_transmission)),
      on_data_directory_(std::move(on_data_directory)),
      on_data_asset_management_(std::move(on_data_asset_management)),
      acquire_state_(std::move(acquire_state)), release_state_(std::move(release_state)),
      on_error_(std::move(on_error)) {}

MmtpParser::~MmtpParser() {
    release_all_states();
}

void MmtpParser::release_all_states() {
    const auto count = signalling_.size() + tracks_.size() + data_assets_.size();
    for (std::size_t index = 0; index < count; ++index) release_state_();
}

void MmtpParser::reset() {
    release_all_states();
    signalling_.clear();
    tracks_.clear();
    data_assets_.clear();
    event_message_tags_.clear();
    ait_packet_ids_.clear();
    data_transmission_packet_ids_.clear();
    committed_mpt_raw_.clear();
    mh_ait_staging_.clear();
    mh_sdt_staging_.clear();
    committed_mh_ait_raw_.clear();
    latest_full_ntp_.reset();
    has_mpt_full_ntp_ = false;
}

void MmtpParser::flush() {
    for (auto& entry : signalling_) {
        auto& assembler = entry.second;
        if (assembler.state == FragmentState::Collecting && !assembler.data.empty()) {
            on_error_(ErrorCode::MalformedInput, 0, true,
                      "dropped incomplete MMTP signalling fragment at end of input");
        }
    }
    for (auto& entry : tracks_) {
        auto& track = entry.second;
        if (track.media.state == FragmentState::Collecting && !track.media.data.empty()) {
            on_error_(ErrorCode::MalformedInput, track.media.input_offset, true,
                      "dropped incomplete MMTP media fragment at end of input");
        }
        finalize_hevc(track);
        if (track.subtitle.active) {
            track.discontinuity = true;
            on_error_(ErrorCode::MalformedInput, track.subtitle.input_offset, true,
                      "dropped incomplete TTML subsample group at end of input");
        }
        track.media = {};
        track.subtitle = {};
        track.current_mpu_sequence.reset();
        track.au_index = 0;
        track.last_emitted_dts.reset();
        track.previous_leap_indicator = 0;
        track.leap_ntp_offset = 0;
        track.leap_examined_mpu.reset();
        track.skipping_hevc_picture = false;
        track.discontinuity = true;
        if (track.info.kind == TrackKind::Video) track.wait_for_rap = true;
    }
    for (auto& entry : data_assets_) {
        auto& asset = entry.second;
        if (asset.media.state == FragmentState::Collecting && !asset.media.data.empty()) {
            on_error_(ErrorCode::MalformedInput, asset.media.input_offset, true,
                      "dropped incomplete non-timed MFU fragment at end of input");
        }
        asset.media = {};
        asset.discontinuity = true;
    }
    for (std::size_t index = 0; index < signalling_.size(); ++index) release_state_();
    signalling_.clear();
}

void MmtpParser::push(const std::uint8_t* data, const std::size_t size,
                      const std::uint64_t input_offset) {
    if (size < 12) {
        on_error_(ErrorCode::MalformedInput, input_offset, true,
                  "MMTP packet is shorter than its fixed header");
        return;
    }

    const auto first = data[0];
    const auto version = static_cast<std::uint8_t>(first >> 6U);
    if (version != 0) {
        on_error_(ErrorCode::UnsupportedFeature, input_offset, true,
                  "unsupported MMTP version");
        return;
    }
    const bool packet_counter_flag = ((first >> 5U) & 1U) != 0;
    const bool extension_header_flag = ((first >> 1U) & 1U) != 0;
    const bool random_access = (first & 1U) != 0;
    const auto payload_type = static_cast<std::uint8_t>(data[1] & 0x3fU);
    const auto packet_id = read_be16(data + 2);
    const auto delivery_timestamp = read_be32(data + 4);
    const auto sequence = read_be32(data + 8);

    std::size_t cursor = 12;
    if (packet_counter_flag) {
        if (size - cursor < 4) {
            on_error_(ErrorCode::MalformedInput, input_offset, true,
                      "truncated MMTP packet counter");
            return;
        }
        cursor += 4;
    }
    PacketExtensions extensions;
    if (extension_header_flag) {
        if (size - cursor < 4) {
            on_error_(ErrorCode::MalformedInput, input_offset, true,
                      "truncated MMTP extension header");
            return;
        }
        const auto extension_type = read_be16(data + cursor);
        const auto extension_size = static_cast<std::size_t>(read_be16(data + cursor + 2));
        cursor += 4;
        if (extension_size > size - cursor) {
            on_error_(ErrorCode::MalformedInput, input_offset, true,
                      "MMTP extension length exceeds packet bounds");
            return;
        }
        if (!parse_packet_extensions(extension_type, data + cursor, extension_size,
                                     extensions)) {
            on_error_(ErrorCode::MalformedInput, input_offset, true,
                      "malformed MMTP multi-type extension header");
            return;
        }
        cursor += extension_size;
    }

    const auto* payload = data + cursor;
    auto payload_size = size - cursor;
    if (extensions.authenticated_payload_size.has_value()) {
        if (*extensions.authenticated_payload_size > payload_size) {
            on_error_(ErrorCode::MalformedInput, input_offset, true,
                      "authenticated MMTP payload length exceeds packet bounds");
            return;
        }
        payload_size = *extensions.authenticated_payload_size;
    }
    if (payload_type == 0x02) {
        parse_signalling(packet_id, sequence, payload, payload_size, input_offset);
    } else if (payload_type == 0x00) {
        parse_mpu(packet_id, sequence, delivery_timestamp, random_access,
                  payload, payload_size, input_offset, extensions);
    } else {
        on_error_(ErrorCode::UnsupportedFeature, input_offset, true,
                  "unsupported MMTP payload type in context " + std::to_string(context_id_));
    }
}


} // namespace aribtlv::detail
