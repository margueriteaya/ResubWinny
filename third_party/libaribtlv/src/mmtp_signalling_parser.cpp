#include "mmtp_parser.hpp"

#include <algorithm>
#include <string>
#include <string_view>
#include <utility>

#include "byte_reader.hpp"

namespace aribtlv::detail {

bool MmtpParser::append(SignallingAssembler& assembler, const std::uint8_t* data,
                        const std::size_t size, const std::uint64_t input_offset) {
    if (size > limits_.max_signalling_message - assembler.data.size()) {
        assembler.data.clear();
        assembler.input_offset = 0;
        assembler.state = FragmentState::Skipping;
        on_error_(ErrorCode::ResourceLimit, input_offset, true,
                  "MMTP signalling message exceeds configured limit");
        return false;
    }
    assembler.data.insert(assembler.data.end(), data, data + size);
    return true;
}

void MmtpParser::accept_signalling_unit(const std::uint16_t packet_id,
                                        const std::uint8_t* data, const std::size_t size,
                                        const std::uint64_t input_offset) {
    if (size < 2) {
        on_error_(ErrorCode::MalformedInput, input_offset, true,
                  "signalling message is too short for a message ID");
        return;
    }
    const auto message_id = read_be16(data);
    bool valid = true;
    if (message_id == 0x0000) {
        valid = parse_pa_message(packet_id, data, size, input_offset);
    } else if (message_id == 0x8000) {
        valid = parse_m2_message(packet_id, data, size, input_offset);
    } else if (message_id == 0x8002) {
        valid = parse_m2_short_message(packet_id, data, size, input_offset);
    } else if (message_id == 0x8003) {
        valid = parse_data_transmission_message(packet_id, data, size, input_offset);
    }
    if (!valid) {
        on_error_(ErrorCode::MalformedInput, input_offset, true,
                  "malformed MMTP signalling message or nested table");
        return;
    }
    SignallingMessage message;
    message.context_id = context_id_;
    message.packet_id = packet_id;
    message.message_id = message_id;
    message.data.assign(data, data + size);
    message.input_offset = input_offset;
    on_signalling_(std::move(message));
}

namespace {

constexpr std::string_view kHdrProgrammeIcon = "\xF0\x9F\x86\xA7";

bool has_hdr_programme_icon(const std::string_view value) noexcept {
    return value.find(kHdrProgrammeIcon) != std::string_view::npos;
}

bool skip_general_location(ByteReader& reader, std::optional<std::uint16_t>& packet_id) {
    std::uint8_t type = 0;
    if (!reader.read_u8(type)) return false;
    switch (type) {
    case 0x00: {
        std::uint16_t value = 0;
        if (!reader.read_u16(value)) return false;
        if (!packet_id.has_value()) packet_id = value;
        return true;
    }
    case 0x01: return reader.skip(12);
    case 0x02: return reader.skip(36);
    case 0x03: return reader.skip(6);
    case 0x04: return reader.skip(38);
    case 0x05: {
        std::uint8_t length = 0;
        return reader.read_u8(length) && reader.skip(length);
    }
    default:
        return false;
    }
}

bool descriptor_length(ByteReader& reader, const std::uint16_t tag, std::uint32_t& length) {
    if (tag <= 0x3fff || (tag >= 0x8000 && tag <= 0xefff)) {
        std::uint8_t value = 0;
        if (!reader.read_u8(value)) return false;
        length = value;
        return true;
    }
    if (tag <= 0x6fff || tag >= 0xf000) {
        std::uint16_t value = 0;
        if (!reader.read_u16(value)) return false;
        length = value;
        return true;
    }
    return reader.read_u32(length);
}

AudioChannelLayout audio_channel_layout(const std::uint8_t component_type) {
    switch (component_type & 0x1fU) {
    case 0x01: return AudioChannelLayout::Mono;
    case 0x02: return AudioChannelLayout::DualMono;
    case 0x03: return AudioChannelLayout::Stereo;
    case 0x04: return AudioChannelLayout::Channels2_1;
    case 0x05: return AudioChannelLayout::Channels3_0;
    case 0x06: return AudioChannelLayout::Channels2_2;
    case 0x07: return AudioChannelLayout::Channels4_0;
    case 0x08: return AudioChannelLayout::Channels5_0;
    case 0x09: return AudioChannelLayout::Channels5_1;
    case 0x0a: return AudioChannelLayout::Channels3_3_1;
    case 0x0b: return AudioChannelLayout::Channels6_1;
    case 0x0c:
    case 0x0d:
    case 0x0e:
    case 0x0f: return AudioChannelLayout::Channels7_1;
    case 0x10: return AudioChannelLayout::Channels10_2;
    case 0x11: return AudioChannelLayout::Channels22_2;
    default: return AudioChannelLayout::Unknown;
    }
}

std::uint32_t audio_sample_rate(const std::uint8_t code) {
    switch (code) {
    case 0x01: return 16000;
    case 0x02: return 22050;
    case 0x03: return 24000;
    case 0x05: return 32000;
    case 0x06: return 44100;
    case 0x07: return 48000;
    default: return 0;
    }
}

bool parse_descriptors(ByteReader& reader, AssetMetadata& metadata,
                       const ErrorCallback& on_error, const std::uint64_t input_offset) {
    while (reader.remaining() != 0) {
        std::uint16_t tag = 0;
        std::uint32_t length = 0;
        if (!reader.read_u16(tag) || !descriptor_length(reader, tag, length) ||
            length > reader.remaining()) {
            return false;
        }
        const std::uint8_t* payload = nullptr;
        if (!reader.read_view(length, payload)) return false;

        if (tag == 0x0001) {
            if (length % 12 != 0) return false;
            ByteReader values(payload, length);
            while (values.remaining() != 0) {
                std::uint32_t sequence = 0;
                const std::uint8_t* ntp = nullptr;
                if (!values.read_u32(sequence) || !values.read_view(8, ntp)) return false;
                metadata.timestamps[sequence] = TimestampMapping{read_be64(ntp)};
            }
        } else if (tag == 0x8000) {
            if (length != 2) return false;
            metadata.asset_groups.push_back(AssetGroupInfo{payload[0], payload[1]});
        } else if (tag == 0x8003) {
            ByteReader values(payload, length);
            while (values.remaining() != 0) {
                MpuPresentationRegion region;
                std::uint8_t reserved_length = 0;
                if (!values.read_u32(region.mpu_sequence_number) ||
                    !values.read_u8(region.layout_number) ||
                    !values.read_u8(region.region_number) ||
                    !values.read_u8(reserved_length) ||
                    !values.skip(reserved_length)) {
                    return false;
                }
                metadata.presentation_regions.push_back(region);
            }
        } else if (tag == 0x800a && length >= 13) {
            if (!metadata.video) metadata.video.emplace();
            metadata.video->hdr_wcg_idc = static_cast<std::uint8_t>(payload[12] & 0x03U);
        } else if (tag == 0x8011 && length >= 2) {
            metadata.component_tag = read_be16(payload);
        } else if (tag == 0x8010 && length >= 8) {
            if (metadata.component_tag == 0) metadata.component_tag = read_be16(payload + 2);
            metadata.language.assign(reinterpret_cast<const char*>(payload + 5), 3);
            if (!metadata.video) metadata.video.emplace();
            metadata.video->video_transfer_characteristics =
                static_cast<std::uint8_t>((payload[4] >> 4U) & 0x0fU);
        } else if (tag == 0x8014 && length >= 10) {
            const auto stream_content = static_cast<std::uint8_t>(payload[0] & 0x0fU);
            const auto stream_type = payload[4];
            const auto flags = payload[6];
            if (metadata.component_tag == 0) {
                metadata.component_tag = read_be16(payload + 2);
            }
            metadata.language.assign(reinterpret_cast<const char*>(payload + 7), 3);
            AudioInfo audio;
            audio.stream_content = stream_content;
            audio.component_type = payload[1];
            audio.component_tag = read_be16(payload + 2);
            audio.channel_layout = audio_channel_layout(audio.component_type);
            audio.stream_type = stream_type;
            audio.simulcast_group_tag = payload[5];
            audio.es_multi_lingual = (flags & 0x80U) != 0;
            audio.main_component = (flags & 0x40U) != 0;
            audio.quality_indicator = static_cast<std::uint8_t>((flags >> 4U) & 0x03U);
            audio.sampling_rate_code = static_cast<std::uint8_t>((flags >> 1U) & 0x07U);
            audio.sample_rate = audio_sample_rate(audio.sampling_rate_code);
            if (audio.es_multi_lingual) {
                if (length < 13) return false;
                audio.secondary_language.assign(reinterpret_cast<const char*>(payload + 10), 3);
            }
            metadata.audio = std::move(audio);
            metadata.aac_latm = stream_content == 0x03 && stream_type == 0x11;
        } else if (tag == 0x8020 && length >= 10 && read_be16(payload) == 0x0020) {
            const auto* additional = payload + 2;
            const auto additional_size = length - 2;
            if (additional_size < 8) return false;
            metadata.language.assign(reinterpret_cast<const char*>(additional + 2), 3);
            SubtitleInfo subtitle;
            subtitle.tag = additional[0];
            subtitle.info_version = static_cast<std::uint8_t>((additional[1] >> 4U) & 0x0fU);
            const bool has_start_mpu_sequence_number = (additional[1] & 0x08U) != 0;
            subtitle.type = static_cast<std::uint8_t>((additional[5] >> 6U) & 0x03U);
            subtitle.format = static_cast<std::uint8_t>((additional[5] >> 2U) & 0x0fU);
            subtitle.operation_mode = static_cast<std::uint8_t>(additional[5] & 0x03U);
            subtitle.timing_mode = static_cast<std::uint8_t>((additional[6] >> 4U) & 0x0fU);
            subtitle.display_mode = static_cast<std::uint8_t>(additional[6] & 0x0fU);
            subtitle.resolution = static_cast<std::uint8_t>((additional[7] >> 4U) & 0x0fU);
            subtitle.compression_type = static_cast<std::uint8_t>(additional[7] & 0x0fU);
            std::size_t offset = 8;
            if (has_start_mpu_sequence_number) {
                if (additional_size < offset + 4) return false;
                subtitle.start_mpu_sequence_number = read_be32(additional + offset);
                offset += 4;
            }
            if (subtitle.timing_mode == 0x02) {
                // reference_start_time (8 bytes) is followed by a 2-bit
                // reference_start_time_leap_indicator and 6 reserved bits, so the
                // conditional block is 9 bytes, not 8.
                if (additional_size < offset + 9) return false;
                subtitle.reference_start_ntp = read_be64(additional + offset);
                subtitle.reference_start_time_leap_indicator =
                    static_cast<std::uint8_t>((additional[offset + 8] >> 6U) & 0x03U);
            }
            metadata.ttml = subtitle.format == 0;
            metadata.subtitle = subtitle;
        } else if (tag == 0x8026 && length >= 1) {
            // ARIB STD-B60 §7.4.3.35 / TR-B39 v2.5-E1 §34.1.3.10 Table 34.1-71: MPU
            // extended timestamp descriptor.
            ByteReader values(payload, length);
            std::uint8_t flags = 0;
            if (!values.read_u8(flags)) return false;
            const auto pts_offset_type = static_cast<std::uint8_t>((flags >> 1U) & 0x03U);
            const bool timescale_present = (flags & 1U) != 0;
            if (timescale_present) {
                std::uint32_t timescale = 0;
                if (!values.read_u32(timescale) || timescale == 0) return false;
                metadata.timescale = timescale;
            }
            std::uint16_t default_pts_offset = 0;
            if (pts_offset_type == 1 && !values.read_u16(default_pts_offset)) return false;
            if (pts_offset_type == 0) {
                on_error(ErrorCode::UnsupportedFeature, input_offset, true,
                         "mpu_extended_timestamp_descriptor omits dts_pts_offset/pts_offset "
                         "(pts_offset_type 0); its access units will have no timestamp mapping");
                continue;
            }
            if (pts_offset_type == 3) {
                on_error(ErrorCode::UnsupportedFeature, input_offset, true,
                         "mpu_extended_timestamp_descriptor: pts_offset_type 3 is reserved by "
                         "TR-B39 Table 34.1-72 and defines no pts_offset semantics; skipping it");
                continue;
            }
            while (values.remaining() != 0) {
                std::uint32_t sequence = 0;
                std::uint8_t leap_and_reserved = 0;
                std::uint16_t decoding_offset = 0;
                std::uint8_t au_count = 0;
                if (!values.read_u32(sequence) || !values.read_u8(leap_and_reserved) ||
                    !values.read_u16(decoding_offset) || !values.read_u8(au_count)) {
                    return false;
                }
                ExtendedTimestampMapping timing;
                timing.pts_offset_type = pts_offset_type;
                timing.leap_indicator = static_cast<std::uint8_t>((leap_and_reserved >> 6U) & 0x03U);
                timing.decoding_time_offset = decoding_offset;
                timing.dts_pts_offsets.reserve(au_count);
                timing.pts_offsets.reserve(au_count);
                for (std::uint16_t index = 0; index < au_count; ++index) {
                    std::uint16_t dts_pts = 0;
                    std::uint16_t pts = default_pts_offset;
                    if (!values.read_u16(dts_pts)) return false;
                    if (pts_offset_type == 2 && !values.read_u16(pts)) return false;
                    timing.dts_pts_offsets.push_back(dts_pts);
                    timing.pts_offsets.push_back(pts);
                }
                metadata.extended_timestamps[sequence] = std::move(timing);
            }
        }
    }
    return true;
}

bool parse_application_service_descriptor(const std::uint32_t context_id,
                                          const std::uint8_t* payload,
                                          const std::size_t length,
                                          ApplicationServiceInfo& info) {
    if (length < 3) return false;
    info.context_id = context_id;
    info.application_format = static_cast<std::uint8_t>(payload[0] >> 4U);
    info.document_resolution = static_cast<std::uint8_t>(payload[1] >> 4U);
    info.default_ait = (payload[2] & 0x80U) != 0;
    info.has_data_transmission_messages = (payload[2] & 0x40U) != 0;
    const auto emt_count = static_cast<std::uint8_t>(payload[2] & 0x0fU);
    ByteReader locations(payload + 3, length - 3);
    if (!skip_general_location(locations, info.ait_packet_id)) return false;
    if (info.has_data_transmission_messages &&
        !skip_general_location(locations, info.data_transmission_packet_id)) {
        return false;
    }
    info.event_message_locations.reserve(emt_count);
    for (std::uint16_t index = 0; index < emt_count; ++index) {
        ApplicationServiceInfo::EventMessageLocation location;
        if (!locations.read_u8(location.event_message_tag) ||
            !skip_general_location(locations, location.packet_id)) {
            return false;
        }
        info.event_message_locations.push_back(std::move(location));
    }
    return true;
}

bool parse_program_descriptors(ByteReader& reader, const std::uint32_t context_id,
                               std::vector<ApplicationServiceInfo>& services) {
    while (reader.remaining() != 0) {
        std::uint16_t tag = 0;
        std::uint32_t length = 0;
        if (!reader.read_u16(tag) || !descriptor_length(reader, tag, length) ||
            length > reader.remaining()) {
            return false;
        }
        const std::uint8_t* payload = nullptr;
        if (!reader.read_view(length, payload)) return false;
        if (tag == 0x8034) {
            ApplicationServiceInfo info;
            if (!parse_application_service_descriptor(context_id, payload, length, info)) {
                return false;
            }
            services.push_back(std::move(info));
        }
    }
    return true;
}

bool parse_application_descriptors(ByteReader& reader, ApplicationInfo& application) {
    while (reader.remaining() != 0) {
        std::uint16_t tag = 0;
        std::uint32_t length = 0;
        if (!reader.read_u16(tag) || !descriptor_length(reader, tag, length) ||
            length > reader.remaining()) {
            return false;
        }
        const std::uint8_t* payload = nullptr;
        if (!reader.read_view(length, payload)) return false;
        if (tag == 0x8029) {
            ByteReader descriptor(payload, length);
            std::uint8_t profiles_length = 0;
            if (!descriptor.read_u8(profiles_length) || profiles_length % 5 != 0 ||
                profiles_length > descriptor.remaining()) {
                return false;
            }
            ByteReader profiles(payload + 1, profiles_length);
            while (profiles.remaining() != 0) {
                ApplicationInfo::Profile profile;
                if (!profiles.read_u16(profile.application_profile) ||
                    !profiles.read_u8(profile.version_major) ||
                    !profiles.read_u8(profile.version_minor) ||
                    !profiles.read_u8(profile.version_micro)) {
                    return false;
                }
                application.profiles.push_back(profile);
            }
            if (!descriptor.skip(profiles_length)) return false;
            std::uint8_t flags = 0;
            if (!descriptor.read_u8(flags) ||
                !descriptor.read_u8(application.application_priority)) {
                return false;
            }
            application.application_descriptor_present = true;
            application.service_bound = (flags & 0x80U) != 0;
            application.visibility = static_cast<std::uint8_t>((flags >> 5U) & 0x03U);
            application.present_application_priority = (flags & 0x01U) != 0;
            while (descriptor.remaining() != 0) {
                std::uint8_t label = 0;
                if (!descriptor.read_u8(label)) return false;
                application.transport_protocol_labels.push_back(label);
            }
        } else if (tag == 0x802b) {
            application.entry_path.assign(reinterpret_cast<const char*>(payload), length);
        } else if (tag == 0x802a && length >= 3) {
            const auto protocol_id = read_be16(payload);
            if (protocol_id != 0x0003 && protocol_id != 0x0005) continue;
            ApplicationInfo::Transport transport;
            transport.protocol_id = protocol_id;
            transport.label = payload[2];
            ByteReader selector(payload + 3, length - 3);
            while (selector.remaining() != 0) {
                std::uint8_t base_length = 0;
                std::vector<std::uint8_t> base;
                std::uint8_t extension_count = 0;
                if (!selector.read_u8(base_length) ||
                    !selector.read_bytes(base_length, base) ||
                    !selector.read_u8(extension_count)) {
                    return false;
                }
                const std::string base_url(reinterpret_cast<const char*>(base.data()), base.size());
                if (extension_count == 0) transport.urls.push_back(base_url);
                for (std::uint16_t index = 0; index < extension_count; ++index) {
                    std::uint8_t extension_length = 0;
                    std::vector<std::uint8_t> extension;
                    if (!selector.read_u8(extension_length) ||
                        !selector.read_bytes(extension_length, extension)) {
                        return false;
                    }
                    transport.urls.push_back(
                        base_url + std::string(reinterpret_cast<const char*>(extension.data()),
                                               extension.size()));
                }
            }
            application.transports.erase(
                std::remove_if(application.transports.begin(), application.transports.end(),
                               [label = transport.label](const auto& existing) {
                                   return existing.label == label;
                               }),
                application.transports.end());
            application.transports.push_back(std::move(transport));
        }
    }
    application.transport_urls.clear();
    for (const auto& transport : application.transports) {
        if (!application.transport_protocol_labels.empty() &&
            std::find(application.transport_protocol_labels.begin(),
                      application.transport_protocol_labels.end(), transport.label) ==
                application.transport_protocol_labels.end()) {
            continue;
        }
        application.transport_urls.insert(application.transport_urls.end(),
                                          transport.urls.begin(), transport.urls.end());
    }
    return true;
}

std::optional<std::uint8_t> decode_bcd(const std::uint8_t value) {
    const auto high = static_cast<std::uint8_t>(value >> 4U);
    const auto low = static_cast<std::uint8_t>(value & 0x0fU);
    if (high > 9 || low > 9) return std::nullopt;
    return static_cast<std::uint8_t>(high * 10 + low);
}

std::optional<std::int64_t> parse_mjd_time(const std::uint8_t* data) {
    if (std::all_of(data, data + 5, [](const std::uint8_t value) { return value == 0xff; })) {
        return std::nullopt;
    }
    const auto hour = decode_bcd(data[2]);
    const auto minute = decode_bcd(data[3]);
    const auto second = decode_bcd(data[4]);
    if (!hour.has_value() || !minute.has_value() || !second.has_value() ||
        *hour > 23 || *minute > 59 || *second > 59) {
        return std::nullopt;
    }
    // MH-EIT expresses the MJD calendar fields in JST. MJD 40587 is
    // 1970-01-01, so subtract nine hours to obtain a Unix UTC timestamp.
    const auto days = static_cast<std::int64_t>(read_be16(data)) - 40587;
    const auto local_seconds = days * 86400 + static_cast<std::int64_t>(*hour) * 3600 +
        static_cast<std::int64_t>(*minute) * 60 + *second;
    return (local_seconds - 9 * 3600) * 1000;
}

std::optional<std::uint32_t> parse_bcd_duration(const std::uint8_t* data) {
    if (data[0] == 0xff && data[1] == 0xff && data[2] == 0xff) return std::nullopt;
    const auto hour = decode_bcd(data[0]);
    const auto minute = decode_bcd(data[1]);
    const auto second = decode_bcd(data[2]);
    if (!hour.has_value() || !minute.has_value() || !second.has_value() ||
        *minute > 59 || *second > 59) {
        return std::nullopt;
    }
    return static_cast<std::uint32_t>(*hour) * 3600U +
        static_cast<std::uint32_t>(*minute) * 60U + *second;
}

std::optional<std::int32_t> parse_bcd_hhmm(const std::uint8_t* data) {
    const auto hour = decode_bcd(data[0]);
    const auto minute = decode_bcd(data[1]);
    if (!hour.has_value() || !minute.has_value() || *hour > 12 || *minute > 59) {
        return std::nullopt;
    }
    return static_cast<std::int32_t>(*hour) * 60 + *minute;
}

bool parse_short_event_descriptor(const std::uint8_t* payload, const std::size_t length,
                                  EventInfo& event) {
    if (length < 6) return false;
    event.language.assign(reinterpret_cast<const char*>(payload), 3);
    std::size_t offset = 3;
    const auto title_length = static_cast<std::size_t>(payload[offset++]);
    if (title_length > length - offset) return false;
    event.title.assign(reinterpret_cast<const char*>(payload + offset), title_length);
    event.hdr_programme_icon = has_hdr_programme_icon(event.title);
    offset += title_length;
    if (length - offset < 2) return false;
    const auto text_length = static_cast<std::size_t>(read_be16(payload + offset));
    offset += 2;
    if (text_length > length - offset) return false;
    event.description.assign(reinterpret_cast<const char*>(payload + offset), text_length);
    return true;
}

} // namespace

bool MmtpParser::parse_pa_message(const std::uint16_t packet_id,
                                  const std::uint8_t* data, const std::size_t size,
                                  const std::uint64_t input_offset) {
    if (size < 7 || read_be16(data) != 0x0000) return false;
    const auto length = static_cast<std::size_t>(read_be32(data + 3));
    if (length > size - 7) return false;
    ByteReader body(data + 7, length);
    std::uint8_t table_count = 0;
    if (!body.read_u8(table_count) || !body.skip(static_cast<std::size_t>(table_count) * 4)) {
        return false;
    }
    return parse_tables(body.current(), body.remaining(), packet_id, input_offset);
}

bool MmtpParser::parse_m2_message(const std::uint16_t packet_id,
                                  const std::uint8_t* data, const std::size_t size,
                                  const std::uint64_t input_offset) {
    if (size < 5 || read_be16(data) != 0x8000) return false;
    const auto length = static_cast<std::size_t>(read_be16(data + 3));
    if (length != size - 5) return false;
    return parse_tables(data + 5, length, packet_id, input_offset);
}

bool MmtpParser::parse_m2_short_message(const std::uint16_t packet_id,
                                        const std::uint8_t* data,
                                        const std::size_t size,
                                        const std::uint64_t input_offset) {
    if (size < 5 || read_be16(data) != 0x8002) return false;
    const auto length = static_cast<std::size_t>(read_be16(data + 3));
    if (length != size - 5) return false;
    return parse_tables(data + 5, length, packet_id, input_offset);
}

bool MmtpParser::parse_data_transmission_message(const std::uint16_t packet_id,
                                                 const std::uint8_t* data,
                                                 const std::size_t size,
                                                 const std::uint64_t input_offset) {
    if (size < 7 || read_be16(data) != 0x8003) return false;
    if (!committed_mpt_raw_.empty() &&
        std::find(data_transmission_packet_ids_.begin(),
                  data_transmission_packet_ids_.end(), packet_id) ==
            data_transmission_packet_ids_.end()) {
        return true;
    }
    const auto length = static_cast<std::size_t>(read_be32(data + 3));
    if (length != size - 7 || length < 12) return false;
    const auto* table = data + 7;
    const auto section_size = 3 + static_cast<std::size_t>(read_be16(table + 1) & 0x0fffU);
    if (section_size != length || section_size < 12) return false;

    DataTransmissionTable result;
    result.context_id = context_id_;
    result.source_packet_id = packet_id;
    result.table_id = table[0];
    result.session_id = table[3];
    result.version = static_cast<std::uint8_t>((table[5] >> 1U) & 0x1fU);
    result.current_next = (table[5] & 0x01U) != 0;
    result.section_number = table[6];
    result.last_section_number = table[7];
    result.data.assign(table, table + section_size);
    result.input_offset = input_offset;
    if (result.table_id == 0xa3 && !parse_data_directory_table(result)) return false;
    if (result.table_id == 0xa4 && !parse_data_asset_management_table(result)) return false;
    on_data_transmission_(std::move(result));
    return true;
}

bool MmtpParser::parse_data_directory_table(const DataTransmissionTable& table) {
    if (table.data.size() < 12) return false;
    ByteReader body(table.data.data() + 8, table.data.size() - 12);
    DataDirectoryTable result;
    result.context_id = table.context_id;
    result.source_packet_id = table.source_packet_id;
    result.session_id = table.session_id;
    result.version = table.version;
    result.current_next = table.current_next;
    result.section_number = table.section_number;
    result.last_section_number = table.last_section_number;
    result.input_offset = table.input_offset;

    std::uint8_t base_path_length = 0;
    std::vector<std::uint8_t> base_path;
    std::uint8_t directory_count = 0;
    if (!body.read_u8(base_path_length) || !body.read_bytes(base_path_length, base_path) ||
        !body.read_u8(directory_count)) {
        return false;
    }
    result.base_path.assign(reinterpret_cast<const char*>(base_path.data()), base_path.size());
    result.directories.reserve(directory_count);
    for (std::uint16_t directory_index = 0; directory_index < directory_count;
         ++directory_index) {
        DataDirectoryNode directory;
        std::uint8_t path_length = 0;
        std::vector<std::uint8_t> path;
        std::uint16_t file_count = 0;
        if (!body.read_u16(directory.node_tag) || !body.read_u8(directory.version) ||
            !body.read_u8(path_length) || !body.read_bytes(path_length, path) ||
            !body.read_u16(file_count)) {
            return false;
        }
        directory.path.assign(reinterpret_cast<const char*>(path.data()), path.size());
        directory.files.reserve(file_count);
        for (std::uint32_t file_index = 0; file_index < file_count; ++file_index) {
            DataDirectoryFile file;
            std::uint8_t name_length = 0;
            std::vector<std::uint8_t> name;
            if (!body.read_u16(file.node_tag) || !body.read_u8(name_length) ||
                !body.read_bytes(name_length, name)) {
                return false;
            }
            file.name.assign(reinterpret_cast<const char*>(name.data()), name.size());
            directory.files.push_back(std::move(file));
        }
        result.directories.push_back(std::move(directory));
    }
    if (body.remaining() != 0) return false;
    on_data_directory_(std::move(result));
    return true;
}

bool MmtpParser::parse_data_asset_management_table(const DataTransmissionTable& table) {
    if (table.data.size() < 23) return false;
    ByteReader body(table.data.data() + 8, table.data.size() - 12);
    DataAssetManagementTable result;
    result.context_id = table.context_id;
    result.source_packet_id = table.source_packet_id;
    result.session_id = table.session_id;
    result.version = table.version;
    result.current_next = table.current_next;
    result.section_number = table.section_number;
    result.last_section_number = table.last_section_number;
    result.input_offset = table.input_offset;
    std::uint8_t mpu_count = 0;
    if (!body.read_u32(result.transaction_id) || !body.read_u16(result.component_tag) ||
        !body.read_u32(result.download_id) || !body.read_u8(mpu_count)) {
        return false;
    }
    result.mpus.reserve(mpu_count);
    for (std::uint16_t mpu_index = 0; mpu_index < mpu_count; ++mpu_index) {
        DataAssetMpu mpu;
        std::uint8_t flags = 0;
        if (!body.read_u32(mpu.sequence_number) || !body.read_u32(mpu.size) ||
            !body.read_u8(flags)) {
            return false;
        }
        mpu.index_item = (flags & 0x80U) != 0;
        const bool index_item_id_present = (flags & 0x40U) != 0;
        mpu.index_item_compression_type = static_cast<std::uint8_t>((flags >> 4U) & 0x03U);
        if (mpu.index_item && index_item_id_present) {
            std::uint32_t item_id = 0;
            if (!body.read_u32(item_id)) return false;
            mpu.index_item_id = item_id;
        }
        std::uint16_t item_count = 0;
        if (!body.read_u16(item_count)) return false;
        mpu.items.reserve(item_count);
        for (std::uint32_t item_index = 0; item_index < item_count; ++item_index) {
            DataAssetItem item;
            if (!body.read_u16(item.node_tag)) return false;
            if (!mpu.index_item) {
                std::uint32_t item_id = 0;
                std::uint32_t item_size = 0;
                std::uint8_t item_version = 0;
                std::uint8_t item_flags = 0;
                if (!body.read_u32(item_id) || !body.read_u32(item_size) ||
                    !body.read_u8(item_version) || !body.read_u8(item_flags)) {
                    return false;
                }
                item.item_id = item_id;
                item.size = item_size;
                item.version = item_version;
                if ((item_flags & 0x80U) != 0) {
                    std::uint32_t checksum = 0;
                    if (!body.read_u32(checksum)) return false;
                    item.checksum = checksum;
                }
                std::uint8_t info_length = 0;
                if (!body.read_u8(info_length) || !body.read_bytes(info_length, item.info)) {
                    return false;
                }
            }
            mpu.items.push_back(std::move(item));
        }
        std::uint8_t mpu_info_length = 0;
        if (!body.read_u8(mpu_info_length) || !body.read_bytes(mpu_info_length, mpu.info)) {
            return false;
        }
        result.mpus.push_back(std::move(mpu));
    }
    std::uint8_t component_info_length = 0;
    if (!body.read_u8(component_info_length) ||
        !body.read_bytes(component_info_length, result.component_info) ||
        body.remaining() != 0) {
        return false;
    }
    on_data_asset_management_(std::move(result));
    return true;
}

bool MmtpParser::parse_mh_ait(const std::uint8_t* data, const std::size_t size,
                              const std::uint16_t packet_id,
                              const std::uint64_t input_offset) {
    if (size < 12 || data[0] != 0x9c) return false;
    if (!committed_mpt_raw_.empty() &&
        std::find(ait_packet_ids_.begin(), ait_packet_ids_.end(), packet_id) ==
            ait_packet_ids_.end()) {
        return true;
    }
    const auto declared_size = 3 + static_cast<std::size_t>(read_be16(data + 1) & 0x0fffU);
    if (declared_size != size) return false;
    ByteReader body(data + 3, size - 3);
    std::uint16_t application_type = 0;
    std::uint8_t version_flags = 0;
    std::uint8_t section_number = 0;
    std::uint8_t last_section_number = 0;
    std::uint16_t common_length_field = 0;
    if (!body.read_u16(application_type) || !body.read_u8(version_flags) ||
        !body.read_u8(section_number) || !body.read_u8(last_section_number) ||
        !body.read_u16(common_length_field)) {
        return false;
    }
    const auto common_length = static_cast<std::size_t>(common_length_field & 0x0fffU);
    const std::uint8_t* common_descriptors = nullptr;
    if (!body.read_view(common_length, common_descriptors)) return false;
    ApplicationInfo common;
    ByteReader common_reader(common_descriptors, common_length);
    if (!parse_application_descriptors(common_reader, common)) return false;
    std::uint16_t loop_length_field = 0;
    if (!body.read_u16(loop_length_field)) return false;
    const auto loop_length = static_cast<std::size_t>(loop_length_field & 0x0fffU);
    const std::uint8_t* loop_data = nullptr;
    if (!body.read_view(loop_length, loop_data) || body.remaining() != 4) return false;

    MhAitSection parsed_section;
    parsed_section.section_number = section_number;
    parsed_section.last_section_number = last_section_number;
    parsed_section.input_offset = input_offset;
    parsed_section.raw.assign(data, data + size);
    ByteReader applications(loop_data, loop_length);
    while (applications.remaining() != 0) {
        if (applications.remaining() < 9) return false;
        ApplicationInfo application = common;
        application.context_id = context_id_;
        application.source_packet_id = packet_id;
        application.application_type = application_type;
        application.version = static_cast<std::uint8_t>((version_flags >> 1U) & 0x1fU);
        application.current_next = (version_flags & 0x01U) != 0;
        application.section_number = section_number;
        application.last_section_number = last_section_number;
        application.input_offset = input_offset;
        std::uint16_t descriptor_length_field = 0;
        if (!applications.read_u16(application.organization_id) ||
            !applications.read_u32(application.application_id) ||
            !applications.read_u8(application.control_code) ||
            !applications.read_u16(descriptor_length_field)) {
            return false;
        }
        const auto descriptor_length =
            static_cast<std::size_t>(descriptor_length_field & 0x0fffU);
        const std::uint8_t* descriptors = nullptr;
        if (!applications.read_view(descriptor_length, descriptors)) return false;
        ByteReader descriptor_reader(descriptors, descriptor_length);
        if (!parse_application_descriptors(descriptor_reader, application)) return false;
        parsed_section.applications.push_back(std::move(application));
    }

    if (section_number > last_section_number) return false;
    const auto current_next = (version_flags & 0x01U) != 0;
    const auto version = static_cast<std::uint8_t>((version_flags >> 1U) & 0x1fU);
    const auto identity = std::to_string(packet_id) + ':' +
        std::to_string(application_type) + ':' + std::to_string(current_next ? 1 : 0);
    auto& assembly = mh_ait_staging_[identity];
    if (!assembly.sections.empty() &&
        (assembly.version != version ||
         assembly.last_section_number != last_section_number)) {
        assembly.sections.clear();
    }
    assembly.version = version;
    assembly.last_section_number = last_section_number;
    const auto existing_section = assembly.sections.find(section_number);
    if (existing_section != assembly.sections.end() &&
        existing_section->second.raw != parsed_section.raw) {
        assembly.sections.clear();
        return false;
    }
    assembly.sections[section_number] = std::move(parsed_section);

    // ARIB-HTML5 operation uses a one-section sub-table numbered 1/1 in real
    // broadcasts.  Treat it as complete without waiting forever for section 0;
    // generic multi-section tables still require the full 0..last set.
    const bool arib_html5_single_section = application_type == 0x0011 &&
        section_number == 1 && last_section_number == 1;
    bool complete = arib_html5_single_section;
    if (!complete && assembly.sections.size() ==
            static_cast<std::size_t>(last_section_number) + 1U) {
        complete = true;
        for (std::uint16_t number = 0; number <= last_section_number; ++number) {
            if (assembly.sections.find(static_cast<std::uint8_t>(number)) ==
                assembly.sections.end()) {
                complete = false;
                break;
            }
        }
    }
    if (!complete) return true;

    std::vector<std::uint8_t> committed_raw;
    MhAitSnapshot snapshot;
    snapshot.context_id = context_id_;
    snapshot.source_packet_id = packet_id;
    snapshot.application_type = application_type;
    snapshot.version = version;
    snapshot.current_next = current_next;
    snapshot.input_offset = assembly.sections.begin()->second.input_offset;
    for (const auto& item : assembly.sections) {
        committed_raw.insert(committed_raw.end(), item.second.raw.begin(), item.second.raw.end());
        snapshot.applications.insert(snapshot.applications.end(),
                                     item.second.applications.begin(),
                                     item.second.applications.end());
    }
    const auto committed = committed_mh_ait_raw_.find(identity);
    if (committed == committed_mh_ait_raw_.end() || committed->second != committed_raw) {
        committed_mh_ait_raw_[identity] = std::move(committed_raw);
        on_mh_ait_snapshot_(std::move(snapshot));
    }
    mh_ait_staging_.erase(identity);
    return true;
}

bool MmtpParser::parse_mh_eit(const std::uint8_t* data, const std::size_t size,
                              const std::uint16_t packet_id,
                              const std::uint64_t input_offset) {
    if (size < 18 || data[0] < 0x8b || data[0] > 0x9b) return false;
    const auto section_length = static_cast<std::size_t>(read_be16(data + 1) & 0x0fffU);
    if (section_length + 3 != size || section_length < 15) return false;

    const auto section_end = size - 4;
    std::size_t offset = 14;
    while (offset < section_end) {
        if (section_end - offset < 12) return false;
        EventInfo event;
        event.context_id = context_id_;
        event.source_packet_id = packet_id;
        event.table_id = data[0];
        event.version = static_cast<std::uint8_t>((data[5] >> 1U) & 0x1fU);
        event.current_next = (data[5] & 0x01U) != 0;
        event.section_number = data[6];
        event.last_section_number = data[7];
        event.service_id = read_be16(data + 3);
        event.tlv_stream_id = read_be16(data + 8);
        event.original_network_id = read_be16(data + 10);
        event.event_id = read_be16(data + offset);
        event.start_time_unix_milliseconds = parse_mjd_time(data + offset + 2);
        event.duration_seconds = parse_bcd_duration(data + offset + 7);
        event.running_status = static_cast<std::uint8_t>(data[offset + 10] >> 5U);
        event.free_ca_mode = (data[offset + 10] & 0x10U) != 0;
        event.input_offset = input_offset;
        const auto descriptors_length =
            static_cast<std::size_t>(read_be16(data + offset + 10) & 0x0fffU);
        offset += 12;
        if (descriptors_length > section_end - offset) return false;

        struct ExtendedFragment {
            std::uint8_t number = 0;
            std::uint8_t last = 0;
            std::string language;
            std::vector<ExtendedEventItem> items;
            std::string text;
        };
        std::vector<ExtendedFragment> extended_fragments;
        ByteReader descriptors(data + offset, descriptors_length);
        while (descriptors.remaining() != 0) {
            std::uint16_t tag = 0;
            std::uint32_t length = 0;
            if (!descriptors.read_u16(tag) || !descriptor_length(descriptors, tag, length) ||
                length > descriptors.remaining()) {
                return false;
            }
            const std::uint8_t* payload = nullptr;
            if (!descriptors.read_view(length, payload)) return false;
            if (tag == 0xf001 && !parse_short_event_descriptor(payload, length, event)) {
                return false;
            } else if (tag == 0xf002) {
                if (length < 8) return false;
                ExtendedFragment fragment;
                fragment.number = static_cast<std::uint8_t>(payload[0] >> 4U);
                fragment.last = static_cast<std::uint8_t>(payload[0] & 0x0fU);
                fragment.language.assign(reinterpret_cast<const char*>(payload + 1), 3);
                const auto items_length = static_cast<std::size_t>(read_be16(payload + 4));
                if (items_length > length - 6) return false;
                std::size_t at = 6;
                const auto items_end = at + items_length;
                while (at < items_end) {
                    const auto description_length = static_cast<std::size_t>(payload[at++]);
                    if (description_length > items_end - at) return false;
                    ExtendedEventItem item;
                    item.description.assign(
                        reinterpret_cast<const char*>(payload + at), description_length);
                    at += description_length;
                    if (items_end - at < 2) return false;
                    const auto item_length = static_cast<std::size_t>(read_be16(payload + at));
                    at += 2;
                    if (item_length > items_end - at) return false;
                    item.value.assign(reinterpret_cast<const char*>(payload + at), item_length);
                    at += item_length;
                    fragment.items.push_back(std::move(item));
                }
                if (length - at < 2) return false;
                const auto text_length = static_cast<std::size_t>(read_be16(payload + at));
                at += 2;
                if (text_length != length - at) return false;
                fragment.text.assign(reinterpret_cast<const char*>(payload + at), text_length);
                extended_fragments.push_back(std::move(fragment));
            } else if (tag == 0x8012) {
                if (length % 2 != 0) return false;
                for (std::size_t at = 0; at < length; at += 2) {
                    event.genres.push_back(ContentGenre{
                        static_cast<std::uint8_t>(payload[at] >> 4U),
                        static_cast<std::uint8_t>(payload[at] & 0x0fU),
                        static_cast<std::uint8_t>(payload[at + 1] >> 4U),
                        static_cast<std::uint8_t>(payload[at + 1] & 0x0fU)});
                }
            } else if (tag == 0x8013) {
                if (length % 4 != 0) return false;
                for (std::size_t at = 0; at < length; at += 4) {
                    ParentalRating rating;
                    rating.country_code.assign(
                        reinterpret_cast<const char*>(payload + at), 3);
                    rating.rating = payload[at + 3];
                    event.parental_ratings.push_back(std::move(rating));
                }
            } else if (tag == 0x8014) {
                if (length < 10) return false;
                EventAudioComponent component;
                component.audio.stream_content = static_cast<std::uint8_t>(payload[0] & 0x0fU);
                component.audio.component_type = payload[1];
                component.audio.component_tag = read_be16(payload + 2);
                component.audio.channel_layout =
                    audio_channel_layout(component.audio.component_type);
                component.audio.stream_type = payload[4];
                component.audio.simulcast_group_tag = payload[5];
                component.audio.es_multi_lingual = (payload[6] & 0x80U) != 0;
                component.audio.main_component = (payload[6] & 0x40U) != 0;
                component.audio.quality_indicator =
                    static_cast<std::uint8_t>((payload[6] >> 4U) & 0x03U);
                component.audio.sampling_rate_code =
                    static_cast<std::uint8_t>((payload[6] >> 1U) & 0x07U);
                component.audio.sample_rate =
                    audio_sample_rate(component.audio.sampling_rate_code);
                component.language.assign(reinterpret_cast<const char*>(payload + 7), 3);
                std::size_t at = 10;
                if (component.audio.es_multi_lingual) {
                    if (length < 13) return false;
                    component.audio.secondary_language.assign(
                        reinterpret_cast<const char*>(payload + 10), 3);
                    at = 13;
                }
                component.text.assign(reinterpret_cast<const char*>(payload + at), length - at);
                event.audio_components.push_back(std::move(component));
            } else if (tag == 0x8016) {
                if (length < 8) return false;
                SeriesInfo series;
                series.series_id = read_be16(payload);
                series.repeat_label = static_cast<std::uint8_t>(payload[2] >> 4U);
                series.program_pattern = static_cast<std::uint8_t>((payload[2] >> 1U) & 0x07U);
                if ((payload[2] & 0x01U) != 0) series.expire_date_mjd = read_be16(payload + 3);
                series.episode_number = static_cast<std::uint16_t>(
                    (static_cast<std::uint16_t>(payload[5]) << 4U) | (payload[6] >> 4U));
                series.last_episode_number = static_cast<std::uint16_t>(
                    ((static_cast<std::uint16_t>(payload[6]) & 0x0fU) << 8U) | payload[7]);
                series.name.assign(reinterpret_cast<const char*>(payload + 8), length - 8);
                event.series = std::move(series);
            }
        }
        if (!extended_fragments.empty()) {
            const auto preferred_language = !event.language.empty()
                ? event.language : extended_fragments.front().language;
            std::sort(extended_fragments.begin(), extended_fragments.end(),
                      [](const auto& left, const auto& right) {
                          return left.number < right.number;
                      });
            std::optional<std::uint8_t> last_number;
            std::optional<std::uint8_t> previous_number;
            for (const auto& fragment : extended_fragments) {
                if (fragment.language != preferred_language) continue;
                if (!last_number.has_value()) last_number = fragment.last;
                if (*last_number != fragment.last ||
                    (previous_number.has_value() && fragment.number == *previous_number)) {
                    return false;
                }
                previous_number = fragment.number;
                event.extended_items.insert(event.extended_items.end(),
                                            fragment.items.begin(), fragment.items.end());
                event.extended_description += fragment.text;
            }
        }
        offset += descriptors_length;
        on_event_(std::move(event));
    }
    return offset == section_end;
}

bool MmtpParser::parse_mh_sdt(const std::uint8_t* data, const std::size_t size,
                              const std::uint16_t packet_id,
                              const std::uint64_t input_offset) {
    if (size < 15 || (data[0] != 0x9f && data[0] != 0xa0)) return false;
    const auto section_length = static_cast<std::size_t>(read_be16(data + 1) & 0x0fffU);
    if (section_length + 3 != size || section_length < 12) return false;
    const auto version = static_cast<std::uint8_t>((data[5] >> 1U) & 0x1fU);
    const bool current_next = (data[5] & 0x01U) != 0;
    const auto section_number = data[6];
    const auto last_section_number = data[7];
    if (section_number > last_section_number) return false;
    if (!current_next) return true;

    MhSdtSection parsed;
    parsed.input_offset = input_offset;
    const auto section_end = size - 4;
    std::size_t offset = 11;
    while (offset < section_end) {
        if (section_end - offset < 5) return false;
        ServiceDescriptionInfo service;
        service.service_id = read_be16(data + offset);
        service.eit_user_defined_flags =
            static_cast<std::uint8_t>((data[offset + 2] >> 2U) & 0x07U);
        service.eit_schedule = (data[offset + 2] & 0x02U) != 0;
        service.eit_present_following = (data[offset + 2] & 0x01U) != 0;
        service.running_status = static_cast<std::uint8_t>(data[offset + 3] >> 5U);
        service.free_ca_mode = (data[offset + 3] & 0x10U) != 0;
        const auto descriptors_length = static_cast<std::size_t>(
            read_be16(data + offset + 3) & 0x0fffU);
        offset += 5;
        if (descriptors_length > section_end - offset) return false;
        ByteReader descriptors(data + offset, descriptors_length);
        while (descriptors.remaining() != 0) {
            std::uint16_t tag = 0;
            std::uint32_t length = 0;
            if (!descriptors.read_u16(tag) ||
                !descriptor_length(descriptors, tag, length) ||
                length > descriptors.remaining()) return false;
            const std::uint8_t* payload = nullptr;
            if (!descriptors.read_view(length, payload)) return false;
            if (tag != 0x8019) continue;
            if (length < 3) return false;
            service.service_type = payload[0];
            std::size_t at = 1;
            const auto provider_length = static_cast<std::size_t>(payload[at++]);
            if (provider_length > length - at) return false;
            service.provider_name.assign(
                reinterpret_cast<const char*>(payload + at), provider_length);
            at += provider_length;
            if (at >= length) return false;
            const auto service_length = static_cast<std::size_t>(payload[at++]);
            if (service_length != length - at) return false;
            service.service_name.assign(
                reinterpret_cast<const char*>(payload + at), service_length);
        }
        offset += descriptors_length;
        parsed.services.push_back(std::move(service));
    }
    if (offset != section_end) return false;

    const auto tlv_stream_id = read_be16(data + 3);
    const auto original_network_id = read_be16(data + 8);
    const auto identity = std::to_string(packet_id) + ':' +
        std::to_string(data[0]) + ':' + std::to_string(tlv_stream_id) + ':' +
        std::to_string(original_network_id);
    auto& assembly = mh_sdt_staging_[identity];
    if (!assembly.sections.empty() &&
        (assembly.version != version ||
         assembly.last_section_number != last_section_number ||
         assembly.tlv_stream_id != tlv_stream_id ||
         assembly.original_network_id != original_network_id)) {
        assembly.sections.clear();
    }
    assembly.version = version;
    assembly.last_section_number = last_section_number;
    assembly.tlv_stream_id = tlv_stream_id;
    assembly.original_network_id = original_network_id;
    const auto existing = assembly.sections.find(section_number);
    if (existing != assembly.sections.end() && existing->second.services != parsed.services) {
        assembly.sections.clear();
        return false;
    }
    assembly.sections[section_number] = std::move(parsed);
    if (assembly.sections.size() != static_cast<std::size_t>(last_section_number) + 1U) {
        return true;
    }
    MhSdtSnapshot snapshot;
    snapshot.context_id = context_id_;
    snapshot.source_packet_id = packet_id;
    snapshot.table_id = data[0];
    snapshot.tlv_stream_id = tlv_stream_id;
    snapshot.original_network_id = original_network_id;
    snapshot.version = version;
    snapshot.current_next = true;
    snapshot.input_offset = assembly.sections.begin()->second.input_offset;
    for (std::uint16_t number = 0; number <= last_section_number; ++number) {
        const auto section = assembly.sections.find(static_cast<std::uint8_t>(number));
        if (section == assembly.sections.end()) return true;
        snapshot.services.insert(snapshot.services.end(), section->second.services.begin(),
                                 section->second.services.end());
    }
    mh_sdt_staging_.erase(identity);
    on_mh_sdt_(std::move(snapshot));
    return true;
}

bool MmtpParser::parse_mh_tot(const std::uint8_t* data, const std::size_t size,
                              const std::uint16_t packet_id,
                              const std::uint64_t input_offset) {
    if (size < 14 || data[0] != 0xa1 || (data[1] & 0x80U) != 0) return false;
    const auto section_length = static_cast<std::size_t>(read_be16(data + 1) & 0x0fffU);
    if (section_length + 3 != size || section_length < 11) return false;
    const auto time = parse_mjd_time(data + 3);
    if (!time.has_value()) return false;
    const auto descriptors_length = static_cast<std::size_t>(read_be16(data + 8) & 0x0fffU);
    if (descriptors_length != size - 14) return false;

    MhTotInfo result;
    result.context_id = context_id_;
    result.source_packet_id = packet_id;
    result.time_unix_milliseconds = *time;
    result.input_offset = input_offset;
    ByteReader descriptors(data + 10, descriptors_length);
    while (descriptors.remaining() != 0) {
        std::uint16_t tag = 0;
        std::uint32_t length = 0;
        if (!descriptors.read_u16(tag) || !descriptor_length(descriptors, tag, length) ||
            length > descriptors.remaining()) return false;
        const std::uint8_t* payload = nullptr;
        if (!descriptors.read_view(length, payload)) return false;
        if (tag != 0x8023) continue;
        if (length % 13 != 0) return false;
        for (std::size_t at = 0; at < length; at += 13) {
            const auto current_offset = parse_bcd_hhmm(payload + at + 4);
            const auto next_offset = parse_bcd_hhmm(payload + at + 11);
            if (!current_offset.has_value() || !next_offset.has_value()) return false;
            LocalTimeOffsetInfo info;
            info.country_code.assign(reinterpret_cast<const char*>(payload + at), 3);
            info.country_region_id = static_cast<std::uint8_t>(payload[at + 3] >> 2U);
            info.polarity = (payload[at + 3] & 0x01U) != 0;
            info.offset_minutes = info.polarity ? *current_offset : -*current_offset;
            info.change_time_unix_milliseconds = parse_mjd_time(payload + at + 6);
            info.next_offset_minutes = info.polarity ? *next_offset : -*next_offset;
            result.local_time_offsets.push_back(std::move(info));
        }
    }
    on_mh_tot_(std::move(result));
    return true;
}

bool MmtpParser::parse_mpt(const std::uint8_t* data, const std::size_t size,
                           const std::uint16_t packet_id,
                           const std::uint64_t input_offset) {
    if (size < 4 || data[0] != 0x20) return false;
    const auto declared_size = static_cast<std::size_t>(read_be16(data + 2));
    if (declared_size != size - 4) return false;
    ByteReader body(data + 4, declared_size);

    std::uint8_t mode = 0;
    std::uint8_t package_length = 0;
    std::vector<std::uint8_t> package_id;
    std::uint16_t program_descriptors_length = 0;
    std::uint8_t asset_count = 0;
    if (!body.read_u8(mode) || !body.read_u8(package_length) ||
        !body.read_bytes(package_length, package_id) ||
        !body.read_u16(program_descriptors_length)) {
        return false;
    }
    const std::uint8_t* program_descriptors = nullptr;
    if (!body.read_view(program_descriptors_length, program_descriptors)) return false;
    ByteReader program_descriptor_reader(program_descriptors, program_descriptors_length);
    std::vector<ApplicationServiceInfo> application_services;
    if (!parse_program_descriptors(program_descriptor_reader, context_id_,
                                   application_services) ||
        !body.read_u8(asset_count)) {
        return false;
    }

    struct ParsedTrack {
        TrackInfo info;
        AssetMetadata metadata;
    };
    std::vector<ParsedTrack> parsed_tracks;
    std::vector<DataAssetInfo> parsed_data_assets;

    for (std::uint16_t asset_index = 0; asset_index < asset_count; ++asset_index) {
        std::uint8_t identifier_type = 0;
        std::uint8_t asset_id_length = 0;
        std::vector<std::uint8_t> asset_id;
        const std::uint8_t* asset_type_data = nullptr;
        std::uint8_t clock_flags = 0;
        std::uint8_t location_count = 0;
        if (!body.read_u8(identifier_type) || !body.skip(4) ||
            !body.read_u8(asset_id_length) || !body.read_bytes(asset_id_length, asset_id) ||
            !body.read_view(4, asset_type_data) || !body.read_u8(clock_flags) ||
            !body.read_u8(location_count)) {
            return false;
        }
        (void)identifier_type;
        (void)clock_flags;

        std::optional<std::uint16_t> packet_id;
        for (std::uint16_t location_index = 0; location_index < location_count; ++location_index) {
            if (!skip_general_location(body, packet_id)) return false;
        }

        std::uint16_t descriptors_length = 0;
        const std::uint8_t* descriptors = nullptr;
        if (!body.read_u16(descriptors_length) ||
            !body.read_view(descriptors_length, descriptors)) {
            return false;
        }
        AssetMetadata metadata;
        ByteReader descriptor_reader(descriptors, descriptors_length);
        if (!parse_descriptors(descriptor_reader, metadata, on_error_, input_offset)) return false;
        if (!packet_id.has_value()) continue;

        const std::string asset_type(reinterpret_cast<const char*>(asset_type_data), 4);
        TrackInfo track;
        track.context_id = context_id_;
        track.packet_id = *packet_id;
        track.asset_id = std::move(asset_id);
        track.language = metadata.language;
        track.component_tag = metadata.component_tag;
        track.timescale = metadata.timescale;
        track.audio = metadata.audio;
        track.subtitle = metadata.subtitle;
        track.asset_groups = metadata.asset_groups;
        track.presentation_regions = metadata.presentation_regions;

        bool supported = true;
        if (asset_type == "hev1") {
            track.kind = TrackKind::Video;
            track.codec = Codec::Hevc;
            track.video = metadata.video;
        } else if (asset_type == "mp4a" && metadata.aac_latm) {
            track.kind = TrackKind::Audio;
            track.codec = Codec::AacLatm;
        } else if (asset_type == "stpp" && metadata.ttml) {
            track.kind = TrackKind::Subtitle;
            track.codec = Codec::Ttml;
            if (track.timescale == 1) track.timescale = 65536;
        } else {
            supported = false;
        }
        if (supported) {
            parsed_tracks.push_back(ParsedTrack{std::move(track), std::move(metadata)});
        } else if (asset_type == "aapp" || asset_type == "asgd" || asset_type == "aagd") {
            DataAssetInfo info;
            info.context_id = context_id_;
            info.packet_id = *packet_id;
            info.asset_id = std::move(track.asset_id);
            info.asset_type = asset_type;
            info.component_tag = metadata.component_tag;
            info.presentation_regions = metadata.presentation_regions;
            parsed_data_assets.push_back(std::move(info));
        }
    }
    if (body.remaining() != 0) return false;

    // Parsing above is deliberately side-effect free.  Only a complete MPT is
    // allowed to replace packet routing and media state.
    std::vector<std::uint16_t> track_packet_ids;
    track_packet_ids.reserve(parsed_tracks.size());
    for (const auto& track : parsed_tracks) track_packet_ids.push_back(track.info.packet_id);
    std::vector<std::uint16_t> data_packet_ids;
    data_packet_ids.reserve(parsed_data_assets.size());
    for (const auto& asset : parsed_data_assets) data_packet_ids.push_back(asset.packet_id);

    for (auto it = tracks_.begin(); it != tracks_.end();) {
        const auto replacement = std::find_if(
            parsed_tracks.begin(), parsed_tracks.end(), [packet = it->first](const auto& value) {
                return value.info.packet_id == packet;
            });
        if (replacement != parsed_tracks.end() &&
            replacement->info.asset_id == it->second.info.asset_id) {
            ++it;
            continue;
        }
        release_state_();
        it = tracks_.erase(it);
    }
    for (auto it = data_assets_.begin(); it != data_assets_.end();) {
        const auto replacement = std::find_if(
            parsed_data_assets.begin(), parsed_data_assets.end(), [packet = it->first](const auto& value) {
                return value.packet_id == packet;
            });
        if (replacement != parsed_data_assets.end() &&
            replacement->asset_id == it->second.info.asset_id) {
            ++it;
            continue;
        }
        release_state_();
        it = data_assets_.erase(it);
    }

    event_message_tags_.clear();
    ait_packet_ids_.clear();
    data_transmission_packet_ids_.clear();
    for (const auto& service : application_services) {
        if (service.ait_packet_id.has_value()) {
            ait_packet_ids_.push_back(*service.ait_packet_id);
        }
        if (service.has_data_transmission_messages &&
            service.data_transmission_packet_id.has_value()) {
            data_transmission_packet_ids_.push_back(*service.data_transmission_packet_id);
        }
        for (const auto& location : service.event_message_locations) {
            if (location.packet_id.has_value()) {
                event_message_tags_[*location.packet_id] = location.event_message_tag;
            }
        }
    }

    for (auto& parsed : parsed_tracks) {
        // A packet cannot remain both a timed track and a data asset after an
        // MPT replacement.
        const auto data = data_assets_.find(parsed.info.packet_id);
        if (data != data_assets_.end()) {
            release_state_();
            data_assets_.erase(data);
        }
        install_track(std::move(parsed.info), std::move(parsed.metadata), input_offset);
    }
    for (const auto& info : parsed_data_assets) {
        const auto track = tracks_.find(info.packet_id);
        if (track != tracks_.end()) {
            release_state_();
            tracks_.erase(track);
        }
        auto state_entry = data_assets_.find(info.packet_id);
        if (state_entry == data_assets_.end()) {
            if (!acquire_state_()) {
                on_error_(ErrorCode::ResourceLimit, input_offset, true,
                          "global MMTP packet/track-state limit exceeded");
                continue;
            }
            state_entry = data_assets_.emplace(info.packet_id, DataAssetState{}).first;
        }
        state_entry->second.info = info;
    }

    MptSnapshot snapshot;
    snapshot.context_id = context_id_;
    snapshot.source_packet_id = packet_id;
    snapshot.package_id = package_id;
    snapshot.version = data[1];
    snapshot.mode = static_cast<std::uint8_t>(mode & 0x03U);
    snapshot.input_offset = input_offset;
    snapshot.application_services = std::move(application_services);
    for (const auto packet : track_packet_ids) {
        const auto found = tracks_.find(packet);
        if (found != tracks_.end()) snapshot.tracks.push_back(found->second.info);
    }
    for (const auto packet : data_packet_ids) {
        const auto found = data_assets_.find(packet);
        if (found != data_assets_.end()) snapshot.data_assets.push_back(found->second.info);
    }
    committed_mpt_raw_.assign(data, data + size);
    on_package_(context_id_, snapshot.package_id);
    on_mpt_snapshot_(std::move(snapshot));
    return true;
}

bool MmtpParser::parse_package_list(const std::uint8_t* data, const std::size_t size,
                                    const std::uint64_t input_offset) {
    (void)input_offset;
    if (size < 4 || data[0] != 0x80) return false;
    const auto declared_size = static_cast<std::size_t>(read_be16(data + 2));
    if (declared_size != size - 4) return false;
    ByteReader body(data + 4, declared_size);
    std::uint8_t package_count = 0;
    if (!body.read_u8(package_count)) return false;
    for (std::uint16_t index = 0; index < package_count; ++index) {
        std::uint8_t package_length = 0;
        std::vector<std::uint8_t> package_id;
        std::optional<std::uint16_t> ignored_packet_id;
        if (!body.read_u8(package_length) || !body.read_bytes(package_length, package_id) ||
            !skip_general_location(body, ignored_packet_id)) {
            return false;
        }
        on_package_(context_id_, std::move(package_id));
    }
    std::uint8_t ip_delivery_count = 0;
    if (!body.read_u8(ip_delivery_count)) return false;
    if (ip_delivery_count != 0) {
        on_error_(ErrorCode::UnsupportedFeature, input_offset, true,
                  "package-list IP delivery alternatives are not supported");
    }
    return true;
}

void MmtpParser::parse_signalling(const std::uint16_t packet_id,
                                  const std::uint32_t sequence,
                                  const std::uint8_t* data, const std::size_t size,
                                  const std::uint64_t input_offset) {
    if (size < 2) {
        on_error_(ErrorCode::MalformedInput, input_offset, true,
                  "truncated MMTP signalling payload header");
        return;
    }
    auto assembler_entry = signalling_.find(packet_id);
    if (assembler_entry == signalling_.end()) {
        if (!acquire_state_()) {
            on_error_(ErrorCode::ResourceLimit, input_offset, true,
                      "global MMTP packet/track-state limit exceeded");
            return;
        }
        assembler_entry = signalling_.emplace(packet_id, SignallingAssembler{}).first;
    }
    auto& assembler = assembler_entry->second;
    const auto flags = data[0];
    const auto fragmentation = static_cast<std::uint8_t>(flags >> 6U);
    const bool length_extension = ((flags >> 1U) & 1U) != 0;
    const bool aggregation = (flags & 1U) != 0;
    const auto* body = data + 2;
    auto body_size = size - 2;

    if (assembler.state != FragmentState::Initial && sequence == assembler.last_sequence) {
        return; // duplicate signalling packet
    }
    if (assembler.state != FragmentState::Initial && sequence != assembler.last_sequence + 1U) {
        if (!assembler.data.empty()) {
            on_error_(ErrorCode::Discontinuity, input_offset, true,
                      "MMTP signalling sequence jump dropped an incomplete unit");
        }
        assembler.data.clear();
        assembler.input_offset = 0;
        assembler.state = FragmentState::Skipping;
    }
    assembler.last_sequence = sequence;

    if (aggregation) {
        if (fragmentation != 0) {
            on_error_(ErrorCode::MalformedInput, input_offset, true,
                      "aggregated signalling payload is also fragmented");
            return;
        }
        const std::size_t length_size = length_extension ? 4 : 2;
        while (body_size != 0) {
            if (body_size < length_size) {
                on_error_(ErrorCode::MalformedInput, input_offset, true,
                          "truncated aggregated signalling length");
                return;
            }
            const auto unit_size = length_extension
                ? static_cast<std::size_t>(read_be32(body))
                : static_cast<std::size_t>(read_be16(body));
            body += length_size;
            body_size -= length_size;
            if (unit_size > body_size || unit_size > limits_.max_signalling_message) {
                on_error_(unit_size > limits_.max_signalling_message
                              ? ErrorCode::ResourceLimit
                              : ErrorCode::MalformedInput,
                          input_offset, true,
                          "aggregated signalling unit length exceeds bounds");
                return;
            }
            accept_signalling_unit(packet_id, body, unit_size, input_offset);
            body += unit_size;
            body_size -= unit_size;
        }
        assembler.state = FragmentState::Idle;
        return;
    }

    switch (fragmentation) {
    case 0:
        if (assembler.state == FragmentState::Collecting) {
            on_error_(ErrorCode::MalformedInput, input_offset, true,
                      "complete signalling unit interrupted a fragmented unit");
        }
        assembler.data.clear();
        assembler.input_offset = 0;
        assembler.state = FragmentState::Idle;
        accept_signalling_unit(packet_id, body, body_size, input_offset);
        break;
    case 1:
        assembler.data.clear();
        assembler.input_offset = input_offset;
        assembler.state = FragmentState::Collecting;
        append(assembler, body, body_size, input_offset);
        break;
    case 2:
        if (assembler.state == FragmentState::Skipping) {
            return;
        }
        if (assembler.state != FragmentState::Collecting) {
            on_error_(ErrorCode::MalformedInput, input_offset, true,
                      "middle signalling fragment has no first fragment");
            assembler.state = FragmentState::Skipping;
            return;
        }
        append(assembler, body, body_size, input_offset);
        break;
    case 3:
        if (assembler.state == FragmentState::Skipping) {
            assembler.state = FragmentState::Idle;
            assembler.data.clear();
            assembler.input_offset = 0;
            return;
        }
        if (assembler.state != FragmentState::Collecting) {
            on_error_(ErrorCode::MalformedInput, input_offset, true,
                      "last signalling fragment has no first fragment");
            return;
        }
        if (append(assembler, body, body_size, input_offset)) {
            accept_signalling_unit(packet_id, assembler.data.data(), assembler.data.size(),
                                   assembler.input_offset);
        }
        assembler.data.clear();
        assembler.input_offset = 0;
        assembler.state = FragmentState::Idle;
        break;
    default:
        break;
    }
}

} // namespace aribtlv::detail
