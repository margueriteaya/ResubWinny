#include "mmtp_parser.hpp"

#include <algorithm>
#include <utility>

#include "byte_reader.hpp"

namespace aribtlv::detail {

namespace {

std::uint64_t expand_short_ntp(const std::uint32_t short_ntp,
                               const std::uint64_t reference) {
    const auto reference_seconds = static_cast<std::int64_t>(reference >> 32U);
    const auto short_seconds = static_cast<std::int64_t>(short_ntp >> 16U);
    auto seconds = (reference_seconds & ~0xffffLL) | short_seconds;
    if (seconds - reference_seconds > 32768) seconds -= 65536;
    if (reference_seconds - seconds > 32768) seconds += 65536;
    const auto fraction = static_cast<std::uint64_t>(short_ntp & 0xffffU) << 16U;
    return (static_cast<std::uint64_t>(seconds) << 32U) | fraction;
}

} // namespace

void MmtpParser::install_track(TrackInfo info, AssetMetadata metadata,
                               const std::uint64_t input_offset) {
    auto state_entry = tracks_.find(info.packet_id);
    if (state_entry == tracks_.end()) {
        if (!acquire_state_()) {
            on_error_(ErrorCode::ResourceLimit, input_offset, true,
                      "global MMTP packet/track-state limit exceeded");
            return;
        }
        state_entry = tracks_.emplace(info.packet_id, TrackState{}).first;
    }
    auto& state = state_entry->second;
    const bool first_install = state.stable_track_id == 0;
    const bool codec_changed = !first_install && state.info.codec != info.codec;
    state.stable_track_id = on_track_(info);
    info.track_id = state.stable_track_id;
    state.info = std::move(info);
    state.restart_offset = input_offset;
    for (auto& entry : metadata.timestamps) {
        entry.second.restart_offset = input_offset;
        state.timestamps[entry.first] = entry.second;
        has_mpt_full_ntp_ = true;
        if (!latest_full_ntp_.has_value() || entry.second.ntp > *latest_full_ntp_) {
            latest_full_ntp_ = entry.second.ntp;
        }
    }
    for (auto& entry : metadata.extended_timestamps) {
        entry.second.restart_offset = input_offset;
        state.extended_timestamps[entry.first] = std::move(entry.second);
    }
    constexpr std::size_t max_timestamp_entries = 32;
    while (state.timestamps.size() > max_timestamp_entries) state.timestamps.erase(state.timestamps.begin());
    while (state.extended_timestamps.size() > max_timestamp_entries) {
        state.extended_timestamps.erase(state.extended_timestamps.begin());
    }
    if ((first_install || codec_changed) && state.info.kind == TrackKind::Video) {
        state.wait_for_rap = true;
        state.skipping_hevc_picture = false;
        state.pending_hevc = {};
        state.media = {};
    }
    // The leap indicator is codec-agnostic (any MPU extended timestamp
    // descriptor may carry it), so this reset is not limited to video. A
    // stale previous_leap_indicator carried across a codec change would let
    // the transition test fire on a stream that never had a leap second.
    if (first_install || codec_changed) {
        state.previous_leap_indicator = 0;
        state.leap_ntp_offset = 0;
        state.leap_examined_mpu.reset();
    }
}

bool MmtpParser::append_media(TrackState& track, const std::uint8_t* data,
                              const std::size_t size, const std::uint64_t input_offset) {
    if (track.media.data.size() > limits_.max_access_unit ||
        size > limits_.max_access_unit - track.media.data.size()) {
        track.media.data.clear();
        track.media.state = FragmentState::Skipping;
        track.discontinuity = true;
        on_error_(ErrorCode::ResourceLimit, input_offset, true,
                  "fragmented MFU exceeds configured access-unit limit");
        return false;
    }
    track.media.data.insert(track.media.data.end(), data, data + size);
    return true;
}

void MmtpParser::consume_mfu_piece(TrackState& track,
                                   const std::uint32_t packet_sequence,
                                   const std::uint32_t mpu_sequence,
                                   const bool timed, const std::uint8_t fragmentation,
                                   const bool aggregation, const bool random_access,
                                   const std::uint8_t* data, const std::size_t size,
                                   const std::uint64_t input_offset) {
    const std::size_t header_size = timed ? 14 : 4;
    if (size < header_size) {
        track.discontinuity = true;
        on_error_(ErrorCode::MalformedInput, input_offset, true,
                  "truncated timed/non-timed MFU header");
        return;
    }
    const auto sample_number = timed ? read_be32(data + 4) : read_be32(data);
    const auto* payload = data + header_size;
    const auto payload_size = size - header_size;
    auto& assembler = track.media;

    if (assembler.state != FragmentState::Initial &&
        packet_sequence != assembler.last_packet_sequence + 1U &&
        !(aggregation && fragmentation == 0 && packet_sequence == assembler.last_packet_sequence)) {
        if (packet_sequence == assembler.last_packet_sequence) {
            return; // duplicate MMTP packet
        }
        if (!assembler.data.empty()) {
            on_error_(ErrorCode::Discontinuity, input_offset, true,
                      "MMTP media sequence jump dropped an incomplete MFU");
        }
        assembler.data.clear();
        assembler.state = FragmentState::Skipping;
        track.discontinuity = true;
    }
    assembler.last_packet_sequence = packet_sequence;

    switch (fragmentation) {
    case 0:
        if (assembler.state == FragmentState::Collecting) {
            assembler.data.clear();
            track.discontinuity = true;
            on_error_(ErrorCode::Discontinuity, input_offset, true,
                      "complete MFU interrupted a fragmented MFU");
        }
        assembler.state = FragmentState::Idle;
        consume_complete_mfu(track, mpu_sequence, timed ? sample_number : 0, random_access,
                             payload, payload_size, input_offset, track.restart_offset);
        break;
    case 1:
        assembler.data.clear();
        assembler.state = FragmentState::Collecting;
        assembler.mpu_sequence = mpu_sequence;
        assembler.sample_number = sample_number;
        assembler.input_offset = input_offset;
        assembler.restart_offset = track.restart_offset;
        assembler.random_access = random_access;
        append_media(track, payload, payload_size, input_offset);
        break;
    case 2:
        if (assembler.state == FragmentState::Skipping) return;
        if (assembler.state != FragmentState::Collecting ||
            assembler.mpu_sequence != mpu_sequence || assembler.sample_number != sample_number) {
            assembler.data.clear();
            assembler.state = FragmentState::Skipping;
            track.discontinuity = true;
            on_error_(ErrorCode::MalformedInput, input_offset, true,
                      "middle MFU fragment has no matching first fragment");
            return;
        }
        assembler.random_access = assembler.random_access || random_access;
        append_media(track, payload, payload_size, input_offset);
        break;
    case 3:
        if (assembler.state == FragmentState::Skipping) {
            assembler.state = FragmentState::Idle;
            assembler.data.clear();
            return;
        }
        if (assembler.state != FragmentState::Collecting ||
            assembler.mpu_sequence != mpu_sequence || assembler.sample_number != sample_number) {
            assembler.data.clear();
            assembler.state = FragmentState::Idle;
            track.discontinuity = true;
            on_error_(ErrorCode::MalformedInput, input_offset, true,
                      "last MFU fragment has no matching first fragment");
            return;
        }
        assembler.random_access = assembler.random_access || random_access;
        if (append_media(track, payload, payload_size, input_offset)) {
            consume_complete_mfu(track, assembler.mpu_sequence,
                                 timed ? assembler.sample_number : 0,
                                 assembler.random_access, assembler.data.data(),
                                 assembler.data.size(), assembler.input_offset,
                                 assembler.restart_offset);
        }
        assembler.data.clear();
        assembler.state = FragmentState::Idle;
        break;
    default:
        break;
    }
}

void MmtpParser::finalize_hevc(TrackState& track) {
    auto& pending = track.pending_hevc;
    if (!pending.active) return;
    if (pending.has_vcl) {
        if (track.wait_for_rap && !pending.random_access) {
            pending = {};
            return;
        }
        if (pending.random_access) track.wait_for_rap = false;
        emit_access_unit(track, pending.mpu_sequence, std::move(pending.data),
                         pending.random_access, pending.input_offset,
                         pending.restart_offset, pending.sample_number);
    } else {
        track.discontinuity = true;
        on_error_(ErrorCode::MalformedInput, pending.input_offset, true,
                  "dropped HEVC access-unit prefix without a VCL NAL unit");
    }
    pending = {};
}

void MmtpParser::consume_complete_mfu(TrackState& track,
                                      const std::uint32_t mpu_sequence,
                                      const std::uint32_t sample_number,
                                      const bool random_access,
                                      const std::uint8_t* data, const std::size_t size,
                                      const std::uint64_t input_offset,
                                      const std::uint64_t restart_offset) {
    if (track.info.codec == Codec::Hevc) {
        if (size < 4 || static_cast<std::size_t>(read_be32(data)) != size - 4) {
            track.discontinuity = true;
            on_error_(ErrorCode::MalformedInput, input_offset, true,
                      "HEVC MFU does not contain one bounded length-prefixed NAL unit");
            return;
        }
        const auto nal_size = size - 4;
        if (nal_size < 2) {
            track.discontinuity = true;
            on_error_(ErrorCode::MalformedInput, input_offset, true,
                      "HEVC NAL unit is shorter than its header");
            return;
        }
        const auto nal_type = static_cast<std::uint8_t>((data[4] >> 1U) & 0x3fU);
        const bool is_vcl = nal_type <= 31;
        const bool is_irap = nal_type >= 16 && nal_type <= 23;
        const bool first_slice = is_vcl && nal_size >= 3 && (data[6] & 0x80U) != 0;
        auto& pending = track.pending_hevc;
        const bool starts_access_unit = nal_type == 35 || first_slice ||
            nal_type == 32 || nal_type == 33 || nal_type == 34 || nal_type == 39;
        if (track.skipping_hevc_picture) {
            if (!starts_access_unit) return;
            track.skipping_hevc_picture = false;
        }
        if (is_vcl && !first_slice && !pending.has_vcl) {
            // Repositioning can restart in the middle of a picture. An IRAP
            // NAL type (or the MMTP RAP flag) on a continuation slice does not
            // make that truncated picture a random-access point: B60 defines
            // RAP_flag in terms of carrying the head of the access point.
            pending = {};
            track.skipping_hevc_picture = true;
            track.discontinuity = true;
            return;
        }
        const bool begins_access_unit = nal_type == 35 ||
            (first_slice && pending.has_vcl) ||
            ((nal_type == 32 || nal_type == 33 || nal_type == 34 || nal_type == 39) &&
             pending.has_vcl) ||
            (sample_number != 0 && pending.active && pending.sample_number != sample_number);
        if (pending.active && begins_access_unit) {
            finalize_hevc(track);
        }
        if (!pending.active) {
            pending.active = true;
            pending.mpu_sequence = mpu_sequence;
            pending.sample_number = sample_number;
            pending.input_offset = input_offset;
            pending.restart_offset = restart_offset;
        }
        if (pending.data.size() > limits_.max_access_unit ||
            limits_.max_access_unit - pending.data.size() < nal_size + 3) {
            pending = {};
            track.discontinuity = true;
            on_error_(ErrorCode::ResourceLimit, input_offset, true,
                      "HEVC decoded access unit exceeds configured limit");
            return;
        }
        pending.random_access = pending.random_access || random_access || is_irap;
        pending.has_vcl = pending.has_vcl || is_vcl;
        pending.data.insert(pending.data.end(), {0x00, 0x00, 0x01});
        pending.data.insert(pending.data.end(), data + 4, data + size);
        return;
    }

    if (track.info.codec == Codec::AacLatm) {
        if (size > 0x1fff) {
            track.discontinuity = true;
            on_error_(ErrorCode::ResourceLimit, input_offset, true,
                      "AAC AudioMuxElement exceeds the 13-bit LOAS length");
            return;
        }
        std::vector<std::uint8_t> loas;
        loas.reserve(size + 3);
        loas.push_back(0x56);
        loas.push_back(static_cast<std::uint8_t>(0xe0U | (size >> 8U)));
        loas.push_back(static_cast<std::uint8_t>(size));
        loas.insert(loas.end(), data, data + size);
        emit_access_unit(track, mpu_sequence, std::move(loas), random_access, input_offset,
                         restart_offset, sample_number);
        return;
    }

    if (track.info.codec != Codec::Ttml || size < 7) {
        track.discontinuity = true;
        on_error_(ErrorCode::MalformedInput, input_offset, true,
                  "truncated or unsupported TTML MFU");
        return;
    }

    const auto subtitle_sequence = data[1];
    const auto subsample_number = data[2];
    const auto last_subsample = data[3];
    const auto flags = data[4];
    const auto data_type = static_cast<std::uint8_t>(flags >> 4U);
    const bool length_extended = ((flags >> 3U) & 1U) != 0;
    const bool info_list = ((flags >> 2U) & 1U) != 0;
    if (data_type > 7 || subsample_number > last_subsample ||
        (subsample_number == 0 && data_type != 0)) {
        track.discontinuity = true;
        on_error_(ErrorCode::UnsupportedFeature, input_offset, true,
                  "unsupported TTML data type or invalid subsample number");
        return;
    }
    std::size_t cursor = 5;
    const std::size_t length_size = length_extended ? 4 : 2;
    if (size - cursor < length_size) {
        track.discontinuity = true;
        on_error_(ErrorCode::MalformedInput, input_offset, true,
                  "truncated TTML data length");
        return;
    }
    const auto data_size = length_extended
        ? static_cast<std::size_t>(read_be32(data + cursor))
        : static_cast<std::size_t>(read_be16(data + cursor));
    cursor += length_size;
    if (subsample_number == 0 && last_subsample > 0 && info_list) {
        for (std::uint16_t index = 0; index < last_subsample; ++index) {
            if (size - cursor < 1 + length_size) {
                track.discontinuity = true;
                on_error_(ErrorCode::MalformedInput, input_offset, true,
                          "truncated TTML subsample information list");
                return;
            }
            const auto listed_data_type = static_cast<std::uint8_t>(data[cursor] >> 4U);
            if (listed_data_type > 7) {
                track.discontinuity = true;
                on_error_(ErrorCode::UnsupportedFeature, input_offset, true,
                          "unsupported TTML resource data type");
                return;
            }
            cursor += 1 + length_size;
        }
    }
    if (data_size > size - cursor || data_size > limits_.max_ttml_sample) {
        track.discontinuity = true;
        on_error_(data_size > limits_.max_ttml_sample
                      ? ErrorCode::ResourceLimit : ErrorCode::MalformedInput,
                  input_offset, true, "TTML subsample length exceeds bounds");
        return;
    }

    auto& subtitle = track.subtitle;
    if (!subtitle.active || subtitle.sequence != subtitle_sequence ||
        subtitle.last_subsample != last_subsample || subtitle.mpu_sequence != mpu_sequence) {
        if (subtitle.active) {
            track.discontinuity = true;
            on_error_(ErrorCode::Discontinuity, input_offset, true,
                      "new TTML unit replaced an incomplete subsample group");
        }
        subtitle = {};
        subtitle.active = true;
        subtitle.sequence = subtitle_sequence;
        subtitle.last_subsample = last_subsample;
        subtitle.mpu_sequence = mpu_sequence;
        subtitle.input_offset = input_offset;
        subtitle.restart_offset = restart_offset;
        subtitle.random_access = random_access;
        subtitle.subsamples.resize(static_cast<std::size_t>(last_subsample) + 1);
    }
    subtitle.random_access = subtitle.random_access || random_access;
    auto& slot = subtitle.subsamples[subsample_number];
    if (!slot.has_value()) {
        slot = SubtitleAssembly::Subsample{
            data_type,
            std::vector<std::uint8_t>(data + cursor, data + cursor + data_size)};
    }
    if (!std::all_of(subtitle.subsamples.begin(), subtitle.subsamples.end(),
                     [](const auto& value) { return value.has_value(); })) {
        return;
    }
    std::size_t total_size = 0;
    for (const auto& value : subtitle.subsamples) total_size += value->data.size();
    if (total_size > limits_.max_ttml_sample) {
        subtitle = {};
        track.discontinuity = true;
        on_error_(ErrorCode::ResourceLimit, input_offset, true,
                  "reassembled TTML sample exceeds configured limit");
        return;
    }
    if (subtitle.subsamples.empty() || !subtitle.subsamples[0].has_value() ||
        subtitle.subsamples[0]->data_type != 0) {
        subtitle = {};
        track.discontinuity = true;
        on_error_(ErrorCode::MalformedInput, input_offset, true,
                  "TTML subtitle group has no document in subsample zero");
        return;
    }
    std::vector<std::uint8_t> ttml = std::move(subtitle.subsamples[0]->data);
    std::vector<SubtitleResource> resources;
    resources.reserve(subtitle.subsamples.size() - 1);
    for (std::size_t index = 1; index < subtitle.subsamples.size(); ++index) {
        auto& value = *subtitle.subsamples[index];
        resources.push_back(SubtitleResource{
            static_cast<std::uint8_t>(index), value.data_type, std::move(value.data)});
    }
    const auto output_offset = subtitle.input_offset;
    const auto output_restart_offset = subtitle.restart_offset;
    const auto output_rap = subtitle.random_access;
    subtitle = {};
    emit_access_unit(track, mpu_sequence, std::move(ttml), output_rap, output_offset,
                     output_restart_offset, 0, std::move(resources));
}

// ARIB STD-B60 §7.4.3.5: mpu_presentation_time is the PTS of the first access unit in
// *presentation* order. ARIB STD-B60 §7.4.3.35 / TR-B39 v2.5-E1 §34.1.3.10 Table 34.1-71
// define the MPU extended timestamp descriptor fields used below:
//   mpu_presentation_time      PTS of the first access unit in presentation order
//   mpu_decoding_time_offset   |DTS(first in decode order) - mpu_presentation_time|
//   dts_pts_offset[i]          PTS(i) - DTS(i), array indexed in decode order
//   pts_offset[j]              PTS gap to the immediately preceding access unit in
//                              presentation order
// The receiver has separate recurrences for the descriptor variants.  Type 1 retains
// its fixed-interval decode-order arithmetic.  For type 2, it starts cumulative PTS at
// zero, gives the first AU a DTS of -mpu_decoding_time_offset, then for each following
// AU uses cumulative PTS minus that AU's dts_pts_offset; PTS is emitted before adding
// that AU's pts_offset.  This is deliberately a decode-order receiver contract: do not
// replace it with a presentation-order reconstruction or reject a decreasing DTS.
void MmtpParser::emit_access_unit(TrackState& track, const std::uint32_t mpu_sequence,
                                  std::vector<std::uint8_t> data,
                                  const bool random_access,
                                  const std::uint64_t input_offset,
                                  const std::uint64_t restart_offset,
                                  const std::uint32_t sample_number,
                                  std::vector<SubtitleResource> subtitle_resources) {
    std::size_t au_index = 0;
    if (sample_number != 0) {
        au_index = static_cast<std::size_t>(sample_number - 1);
        if (track.au_index <= au_index) track.au_index = au_index + 1;
    } else {
        au_index = track.au_index;
        ++track.au_index;
    }
    const auto timestamp = track.timestamps.find(mpu_sequence);
    const auto extended = track.extended_timestamps.find(mpu_sequence);
    std::int64_t dts_offset = 0;
    std::int64_t pts_offset = 0;
    std::uint64_t ntp = 0;
    auto output_restart_offset = restart_offset;
    if (timestamp != track.timestamps.end() && extended != track.extended_timestamps.end() &&
        au_index < extended->second.dts_pts_offsets.size() &&
        au_index < extended->second.pts_offsets.size()) {
        output_restart_offset = std::min(
            output_restart_offset,
            std::min(timestamp->second.restart_offset,
                     extended->second.restart_offset));
        if (extended->second.pts_offset_type == 2) {
            for (std::size_t index = 0; index < au_index; ++index) {
                pts_offset += extended->second.pts_offsets[index];
            }
            dts_offset = au_index == 0
                ? -static_cast<std::int64_t>(extended->second.decoding_time_offset)
                : pts_offset - static_cast<std::int64_t>(extended->second.dts_pts_offsets[au_index]);
        } else {
            dts_offset = -static_cast<std::int64_t>(extended->second.decoding_time_offset) +
                static_cast<std::int64_t>(au_index) * extended->second.pts_offsets[0];
            pts_offset = dts_offset + extended->second.dts_pts_offsets[au_index];
            if (track.last_emitted_dts.has_value() && dts_offset < *track.last_emitted_dts) {
                track.discontinuity = true;
                on_error_(ErrorCode::MalformedInput, input_offset, true,
                          "dropped access unit with a decreasing decode timestamp inside an MPU");
                return;
            }
            track.last_emitted_dts = dts_offset;
        }
        // TR-B39 Appendix 1 Chapter 2 (no receiver clock leap adjustment):
        // watch mpu_presentation_time_leap_indicator transitions once per MPU
        // and fold them into a persistent, cumulative correction of the NTP
        // anchor. Per-AU dts/pts offsets above are transmitted without leap
        // adjustment and are left untouched.
        if (!track.leap_examined_mpu.has_value() || *track.leap_examined_mpu != mpu_sequence) {
            const auto leap_indicator = extended->second.leap_indicator;
            if (track.previous_leap_indicator == 1 && leap_indicator == 0) {
                track.leap_ntp_offset += std::int64_t{1} << 32U;
            } else if (track.previous_leap_indicator == 2 && leap_indicator == 0) {
                track.leap_ntp_offset -= std::int64_t{1} << 32U;
            }
            track.previous_leap_indicator = leap_indicator;
            track.leap_examined_mpu = mpu_sequence;
        }
        // Deliberately corrects unit.source_ntp too, not just the media
        // timeline: TR-B39 Appendix 1 section 2.1 documents this model as
        // leaving the receiver a full second "asynchronous with respect to
        // the sending system clock" by design, so the broadcast clock must
        // carry the same offset as the presentation timeline it is derived
        // from.
        ntp = static_cast<std::uint64_t>(
            static_cast<std::int64_t>(timestamp->second.ntp) + track.leap_ntp_offset);
    } else if (track.info.codec == Codec::Ttml && latest_full_ntp_.has_value()) {
        const auto delivery = track.delivery_timestamps.find(mpu_sequence);
        if (delivery == track.delivery_timestamps.end()) {
            track.discontinuity = true;
            on_error_(ErrorCode::Discontinuity, input_offset, true,
                      "dropped TTML sample without a delivery timestamp");
            return;
        }
        // B60 provides no MPU timestamp descriptors on the subtitle assets in
        // the broadcast samples. Their timed MPU is therefore anchored to the
        // MMTP short-form NTP delivery timestamp, expanded around the latest
        // full NTP mapping received for the same context.
        ntp = expand_short_ntp(delivery->second, *latest_full_ntp_);
    } else {
        track.discontinuity = true;
        on_error_(ErrorCode::Discontinuity, input_offset, true,
                  "dropped access unit without a matching timestamp descriptor");
        return;
    }

    const auto ntp_seconds = ntp >> 32U;
    const auto ntp_fraction = static_cast<std::uint32_t>(ntp);
    const auto ntp_microseconds = static_cast<std::int64_t>(
        ntp_seconds * 1000000ULL +
        (static_cast<std::uint64_t>(ntp_fraction) * 1000000ULL >> 32U));

    AccessUnit unit;
    unit.track_id = track.stable_track_id;
    unit.codec = track.info.codec;
    unit.component_tag = track.info.component_tag;
    if (track.info.subtitle.has_value()) {
        unit.subtitle_timing_mode = track.info.subtitle->timing_mode;
        unit.subtitle_operation_mode = track.info.subtitle->operation_mode;
        unit.subtitle_display_mode = track.info.subtitle->display_mode;
        unit.subtitle_compression_type = track.info.subtitle->compression_type;
    }
    unit.data = std::move(data);
    unit.subtitle_resources = std::move(subtitle_resources);
    unit.pts = Timestamp{pts_offset, track.info.timescale};
    unit.dts = Timestamp{dts_offset, track.info.timescale};
    unit.source_ntp = Timestamp{ntp_microseconds, 1000000};
    unit.mpu_sequence_number = mpu_sequence;
    unit.restart_offset = output_restart_offset;
    unit.input_offset = input_offset;
    unit.random_access = random_access;
    unit.discontinuity = track.discontinuity;
    if (track.discontinuity) {
        unit.discontinuity_reasons = DiscontinuityReason::SourceDamage;
    }
    track.discontinuity = false;
    on_access_unit_(TimedAccessUnit{std::move(unit), ntp});
}

void MmtpParser::emit_data_unit(DataAssetState& asset,
                                const std::uint32_t mpu_sequence,
                                const std::uint32_t item_id,
                                const std::uint8_t* data, const std::size_t size,
                                const std::uint64_t input_offset,
                                const PacketExtensions& extensions) {
    DataUnit unit;
    unit.context_id = context_id_;
    unit.packet_id = asset.info.packet_id;
    unit.asset_id = asset.info.asset_id;
    unit.asset_type = asset.info.asset_type;
    unit.component_tag = asset.info.component_tag;
    unit.mpu_sequence_number = mpu_sequence;
    unit.item_id = item_id;
    unit.download_id = extensions.download_id;
    unit.item_fragment_number = extensions.item_fragment_number;
    unit.last_item_fragment_number = extensions.last_item_fragment_number;
    unit.data.assign(data, data + size);
    unit.input_offset = input_offset;
    unit.discontinuity = asset.discontinuity;
    asset.discontinuity = false;
    on_data_unit_(std::move(unit));
}

void MmtpParser::consume_data_piece(DataAssetState& asset,
                                    const std::uint32_t packet_sequence,
                                    const std::uint32_t mpu_sequence,
                                    const std::uint8_t fragmentation,
                                    const bool aggregation,
                                    const std::uint8_t* data, const std::size_t size,
                                    const std::uint64_t input_offset,
                                    const PacketExtensions& extensions) {
    if (size < 4) {
        asset.discontinuity = true;
        on_error_(ErrorCode::MalformedInput, input_offset, true,
                  "truncated non-timed MFU header");
        return;
    }
    const auto item_id = read_be32(data);
    const auto* payload = data + 4;
    const auto payload_size = size - 4;
    auto& assembler = asset.media;
    if (assembler.state != FragmentState::Initial &&
        packet_sequence != assembler.last_packet_sequence + 1U &&
        !(aggregation && fragmentation == 0 &&
          packet_sequence == assembler.last_packet_sequence)) {
        if (packet_sequence == assembler.last_packet_sequence) return;
        if (!assembler.data.empty()) {
            on_error_(ErrorCode::Discontinuity, input_offset, true,
                      "MMTP data sequence jump dropped an incomplete MFU");
        }
        assembler.data.clear();
        assembler.state = FragmentState::Skipping;
        asset.discontinuity = true;
    }
    assembler.last_packet_sequence = packet_sequence;
    auto append_data = [&]() {
        if (assembler.data.size() > limits_.max_access_unit ||
            payload_size > limits_.max_access_unit - assembler.data.size()) {
            assembler.data.clear();
            assembler.state = FragmentState::Skipping;
            asset.discontinuity = true;
            on_error_(ErrorCode::ResourceLimit, input_offset, true,
                      "fragmented data MFU exceeds configured access-unit limit");
            return false;
        }
        assembler.data.insert(assembler.data.end(), payload, payload + payload_size);
        return true;
    };
    switch (fragmentation) {
    case 0:
        if (assembler.state == FragmentState::Collecting) {
            assembler.data.clear();
            asset.discontinuity = true;
        }
        assembler.state = FragmentState::Idle;
        emit_data_unit(asset, mpu_sequence, item_id, payload, payload_size,
                       input_offset, extensions);
        break;
    case 1:
        assembler.data.clear();
        assembler.state = FragmentState::Collecting;
        assembler.mpu_sequence = mpu_sequence;
        assembler.sample_number = item_id;
        assembler.input_offset = input_offset;
        assembler.download_id = extensions.download_id;
        assembler.item_fragment_number = extensions.item_fragment_number;
        assembler.last_item_fragment_number = extensions.last_item_fragment_number;
        append_data();
        break;
    case 2:
        if (assembler.state == FragmentState::Skipping) return;
        if (assembler.state != FragmentState::Collecting ||
            assembler.mpu_sequence != mpu_sequence || assembler.sample_number != item_id) {
            assembler.data.clear();
            assembler.state = FragmentState::Skipping;
            asset.discontinuity = true;
            return;
        }
        append_data();
        break;
    case 3:
        if (assembler.state == FragmentState::Skipping) {
            assembler.state = FragmentState::Idle;
            assembler.data.clear();
            return;
        }
        if (assembler.state != FragmentState::Collecting ||
            assembler.mpu_sequence != mpu_sequence || assembler.sample_number != item_id) {
            assembler.data.clear();
            assembler.state = FragmentState::Idle;
            asset.discontinuity = true;
            return;
        }
        if (append_data()) {
            PacketExtensions collected;
            collected.download_id = assembler.download_id;
            collected.item_fragment_number = assembler.item_fragment_number;
            collected.last_item_fragment_number = assembler.last_item_fragment_number;
            emit_data_unit(asset, assembler.mpu_sequence, assembler.sample_number,
                           assembler.data.data(), assembler.data.size(),
                           assembler.input_offset, collected);
        }
        assembler.data.clear();
        assembler.state = FragmentState::Idle;
        break;
    default:
        break;
    }
}

void MmtpParser::parse_mpu(const std::uint16_t packet_id,
                           const std::uint32_t packet_sequence,
                           const std::uint32_t delivery_timestamp,
                           const bool random_access, const std::uint8_t* data,
                           const std::size_t size, const std::uint64_t input_offset,
                           const PacketExtensions& extensions) {
    if (size < 8) {
        on_error_(ErrorCode::MalformedInput, input_offset, true,
                  "truncated MMTP MPU payload");
        return;
    }
    const auto declared_size = static_cast<std::size_t>(read_be16(data));
    if (declared_size != size - 2) {
        on_error_(ErrorCode::MalformedInput, input_offset, true,
                  "MMTP MPU payload length does not match its container");
        return;
    }
    const auto fragment_type = static_cast<std::uint8_t>(data[2] >> 4U);
    if (fragment_type != 2) {
        return;
    }
    const auto flags = data[2];
    const bool timed = ((flags >> 3U) & 1U) != 0;
    const auto fragmentation = static_cast<std::uint8_t>((flags >> 1U) & 0x03U);
    const bool aggregation = (flags & 1U) != 0;
    const auto mpu_sequence = read_be32(data + 4);
    const auto data_asset_entry = data_assets_.find(packet_id);
    if (data_asset_entry != data_assets_.end()) {
        auto& asset = data_asset_entry->second;
        if (timed) {
            asset.discontinuity = true;
            on_error_(ErrorCode::UnsupportedFeature, input_offset, true,
                      "timed data-asset MFU is unsupported");
            return;
        }
        if (aggregation && fragmentation != 0) {
            asset.discontinuity = true;
            on_error_(ErrorCode::MalformedInput, input_offset, true,
                      "aggregated data MPU payload is also fragmented");
            return;
        }
        const auto* body = data + 8;
        auto body_size = size - 8;
        if (!aggregation) {
            consume_data_piece(asset, packet_sequence, mpu_sequence, fragmentation,
                               false, body, body_size, input_offset, extensions);
            return;
        }
        while (body_size != 0) {
            if (body_size < 2) return;
            const auto unit_size = static_cast<std::size_t>(read_be16(body));
            body += 2;
            body_size -= 2;
            if (unit_size > body_size) return;
            consume_data_piece(asset, packet_sequence, mpu_sequence, 0, true,
                               body, unit_size, input_offset, extensions);
            body += unit_size;
            body_size -= unit_size;
        }
        return;
    }

    const auto track_entry = tracks_.find(packet_id);
    if (track_entry == tracks_.end()) return;
    auto& track = track_entry->second;
    track.delivery_timestamps[mpu_sequence] = delivery_timestamp;
    while (track.delivery_timestamps.size() > 32) {
        track.delivery_timestamps.erase(track.delivery_timestamps.begin());
    }

    if (aggregation && fragmentation != 0) {
        track.discontinuity = true;
        on_error_(ErrorCode::MalformedInput, input_offset, true,
                  "aggregated MPU payload is also fragmented");
        return;
    }
    if (!track.current_mpu_sequence.has_value() || *track.current_mpu_sequence != mpu_sequence) {
        if (track.current_mpu_sequence.has_value()) {
            finalize_hevc(track);
            const auto previous = track.extended_timestamps.find(*track.current_mpu_sequence);
            if (!track.wait_for_rap && previous != track.extended_timestamps.end() &&
                track.au_index != previous->second.dts_pts_offsets.size()) {
                track.discontinuity = true;
                on_error_(ErrorCode::Discontinuity, input_offset, true,
                          "MPU access-unit count disagrees with its timestamp descriptor");
            }
            if (mpu_sequence != *track.current_mpu_sequence + 1U) track.discontinuity = true;
            if (track.subtitle.active) {
                track.subtitle = {};
                track.discontinuity = true;
            }
        }
        track.current_mpu_sequence = mpu_sequence;
        track.au_index = 0;
        track.last_emitted_dts.reset();
        // The cumulative leap offset persists for the service; only the
        // once-per-MPU examination guard advances here.
        track.leap_examined_mpu.reset();
    }

    const auto* body = data + 8;
    auto body_size = size - 8;
    if (!aggregation) {
        consume_mfu_piece(track, packet_sequence, mpu_sequence, timed, fragmentation,
                          false, random_access, body, body_size, input_offset);
        return;
    }
    while (body_size != 0) {
        if (body_size < 2) {
            track.discontinuity = true;
            on_error_(ErrorCode::MalformedInput, input_offset, true,
                      "truncated aggregated MFU length");
            return;
        }
        const auto unit_size = static_cast<std::size_t>(read_be16(body));
        body += 2;
        body_size -= 2;
        if (unit_size > body_size) {
            track.discontinuity = true;
            on_error_(ErrorCode::MalformedInput, input_offset, true,
                      "aggregated MFU length exceeds MPU payload");
            return;
        }
        consume_mfu_piece(track, packet_sequence, mpu_sequence, timed, 0,
                          true, random_access, body, unit_size, input_offset);
        body += unit_size;
        body_size -= unit_size;
    }
}

} // namespace aribtlv::detail
