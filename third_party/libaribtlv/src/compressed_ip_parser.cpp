#include "compressed_ip_parser.hpp"

#include <utility>

namespace aribtlv::detail {

CompressedIpParser::CompressedIpParser(const Limits& limits, ServiceCallback on_service,
                                       TrackCallback on_track, AccessUnitCallback on_access_unit,
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
                                       FlowCallback on_flow,
                                       NtpCallback on_ntp,
                                       NitCallback on_nit,
                                       AddressMapCallback on_address_map,
                                       RawTableCallback on_raw_table,
                                       UnknownDescriptorCallback on_unknown_descriptor,
                                       ErrorCallback on_error)
    : limits_(limits), on_service_(std::move(on_service)), on_track_(std::move(on_track)),
      on_access_unit_(std::move(on_access_unit)),
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
      on_error_(std::move(on_error)),
      transport_(std::move(on_flow),
                 [this, callback = std::move(on_ntp)](TransportNtpClock clock) {
                     for (auto& [id, parser] : contexts_) {
                         (void)id;
                         parser->seed_full_ntp(clock.transmit_timestamp);
                     }
                     callback(std::move(clock));
                 },
                 std::move(on_nit), std::move(on_address_map),
                 std::move(on_raw_table), std::move(on_unknown_descriptor),
                 on_error_) {}

void CompressedIpParser::reset() {
    contexts_.clear();
    active_packet_states_ = 0;
    transport_.reset();
}

void CompressedIpParser::select_service(std::optional<std::uint32_t> context_id) {
    if (selected_service_ == context_id) return;
    selected_service_ = context_id;
    reset();
}

void CompressedIpParser::flush() {
    for (auto& entry : contexts_) {
        entry.second->flush();
    }
}

MmtpParser* CompressedIpParser::context(const std::uint32_t context_id,
                                        const std::uint64_t input_offset) {
    const auto found = contexts_.find(context_id);
    if (found != contexts_.end()) {
        return found->second.get();
    }
    if (contexts_.size() >= limits_.max_contexts) {
        on_error_(ErrorCode::ResourceLimit, input_offset, true,
                  "compressed-IP context limit exceeded");
        return nullptr;
    }
    auto parser = std::make_unique<MmtpParser>(
        context_id, limits_,
        [this](const std::uint32_t id, std::vector<std::uint8_t> package_id) {
            on_service_(ServiceInfo{id, std::move(package_id)});
        },
        on_track_, on_access_unit_, on_application_service_, on_layout_,
        on_data_asset_, on_data_unit_,
        on_signalling_, on_event_, on_mh_sdt_, on_mh_tot_,
        on_stream_event_, on_viewer_participation_,
        on_application_, on_mpt_snapshot_, on_mh_ait_snapshot_, on_data_transmission_,
        on_data_directory_, on_data_asset_management_,
        [this]() {
            if (active_packet_states_ >= limits_.max_packet_states) return false;
            ++active_packet_states_;
            return true;
        },
        [this]() {
            if (active_packet_states_ != 0) --active_packet_states_;
        },
        on_error_);
    auto* result = parser.get();
    if (const auto ntp = transport_.latest_ntp()) result->seed_full_ntp(*ntp);
    contexts_.emplace(context_id, std::move(parser));
    on_service_(ServiceInfo{context_id, {}});
    return result;
}

void CompressedIpParser::consume(const TlvPacketView& packet) {
    switch (packet.type) {
    case 0x01:
        transport_.consume_ipv4(packet);
        break;
    case 0x02:
        transport_.consume_ipv6(packet);
        break;
    case 0x03:
        parse_compressed(packet);
        break;
    case 0xfe:
        transport_.consume_tlv_si(packet);
        break;
    case 0xff:
        transport_.consume_null(packet);
        break;
    default:
        on_error_(ErrorCode::UnsupportedFeature, packet.input_offset, true,
                  "unsupported TLV packet type");
        break;
    }
}

void CompressedIpParser::parse_compressed(const TlvPacketView& packet) {
    if (packet.size < 3) {
        on_error_(ErrorCode::MalformedInput, packet.input_offset, true,
                  "truncated compressed-IP header");
        return;
    }
    const auto context_and_sequence = read_be16(packet.payload);
    const auto context_id = static_cast<std::uint32_t>(context_and_sequence >> 4U);
    const auto sequence_number = static_cast<std::uint8_t>(context_and_sequence & 0x0fU);
    if (selected_service_.has_value() && *selected_service_ != context_id) return;
    const auto mode = packet.payload[2];
    std::size_t cursor = 3;
    std::size_t compressed_header_size = 0;
    if (mode == 0x20) {
        // Existing compatibility layout: partial IPv4 (16 bytes) and
        // partial UDP (4 bytes). This mode is not part of the receiver-derived
        // TLV transport contract.
        compressed_header_size = 16 + 4;
    } else if (mode == 0x21) {
        compressed_header_size = 2; // IPv4 identifier
    } else if (mode == 0x60) {
        compressed_header_size = 38 + 4; // partial IPv6 and partial UDP
    } else if (mode != 0x61) {
        on_error_(ErrorCode::MalformedInput, packet.input_offset, true,
                  "unsupported compressed-IP context identification mode");
        return;
    }
    if (mode == 0x60 &&
        !transport_.observe_compressed_flow(packet, context_id, sequence_number)) return;
    if (packet.size - cursor < compressed_header_size) {
        on_error_(ErrorCode::MalformedInput, packet.input_offset, true,
                  "truncated compressed-IP context header");
        return;
    }
    cursor += compressed_header_size;

    auto* parser = context(context_id, packet.input_offset);
    if (parser == nullptr) {
        return;
    }
    parser->push(packet.payload + cursor, packet.size - cursor, packet.input_offset);
}

} // namespace aribtlv::detail
