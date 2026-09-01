#include "tlv_transport_parser.hpp"

#include <algorithm>
#include <array>
#include <limits>
#include <string>
#include <utility>

#include "parser_common.hpp"

namespace aribtlv::detail {
namespace {

std::uint32_t crc32_mpeg(const std::uint8_t* data, const std::size_t size) {
    std::uint32_t crc = 0xffffffffU;
    for (std::size_t index = 0; index < size; ++index) {
        crc ^= static_cast<std::uint32_t>(data[index]) << 24U;
        for (unsigned bit = 0; bit < 8; ++bit) {
            crc = (crc & 0x80000000U) != 0
                ? (crc << 1U) ^ 0x04c11db7U
                : crc << 1U;
        }
    }
    return crc;
}

bool known_tlv_descriptor(const std::uint8_t tag) noexcept {
    switch (tag) {
    case 0x40: // network name
    case 0x41: // service list
    case 0x43: // satellite delivery system
    case 0xcd: // remote control key
    case 0xfe: // system management
        return true;
    default:
        return false;
    }
}

std::uint64_t read_be64_local(const std::uint8_t* data) noexcept {
    std::uint64_t value = 0;
    for (unsigned index = 0; index < 8; ++index) {
        value = (value << 8U) | data[index];
    }
    return value;
}

std::int64_t expand_ntp_microseconds(const std::uint64_t timestamp) noexcept {
    std::uint64_t seconds = timestamp >> 32U;
    if ((seconds & 0x80000000ULL) == 0) seconds += 1ULL << 32U;
    const auto fraction = static_cast<std::uint32_t>(timestamp);
    const auto micros = seconds * 1000000ULL +
        (static_cast<std::uint64_t>(fraction) * 1000000ULL >> 32U);
    return static_cast<std::int64_t>(micros);
}

} // namespace

TlvTransportParser::TlvTransportParser(
    FlowCallback on_flow, NtpCallback on_ntp, NitCallback on_nit,
    AddressMapCallback on_address_map, RawTableCallback on_raw_table,
    UnknownDescriptorCallback on_unknown_descriptor, ErrorCallback on_error)
    : on_flow_(std::move(on_flow)), on_ntp_(std::move(on_ntp)),
      on_nit_(std::move(on_nit)), on_address_map_(std::move(on_address_map)),
      on_raw_table_(std::move(on_raw_table)),
      on_unknown_descriptor_(std::move(on_unknown_descriptor)),
      on_error_(std::move(on_error)) {}

void TlvTransportParser::reset() {
    flows_.clear();
    section_buffer_.clear();
    section_origins_.clear();
    nit_assemblies_.clear();
    address_map_assemblies_.clear();
    latest_ntp_.reset();
    null_packet_count_ = 0;
}

void TlvTransportParser::report(const ErrorCode code, const std::uint64_t offset,
                                std::string message) {
    on_error_(code, offset, true, std::move(message));
}

void TlvTransportParser::consume_ipv4(const TlvPacketView& packet) {
    report(ErrorCode::UnsupportedFeature, packet.input_offset,
           "IPv4 TLV packet is recognized but receiver-compatible parsing is unavailable");
}

void TlvTransportParser::consume_ipv6(const TlvPacketView& packet) {
    if (packet.size < 48 || (packet.payload[0] >> 4U) != 6) {
        report(ErrorCode::MalformedInput, packet.input_offset,
               "truncated or invalid IPv6 TLV payload");
        return;
    }
    const auto payload_length = static_cast<std::size_t>(read_be16(packet.payload + 4));
    if (payload_length != packet.size - 40) {
        report(ErrorCode::MalformedInput, packet.input_offset,
               "IPv6 payload length does not match TLV payload");
        return;
    }

    const auto packet_end = packet.size;
    std::size_t offset = 40;
    auto next_header = packet.payload[6];
    for (unsigned count = 0; count < 16 && next_header != 17; ++count) {
        if (next_header == 0 || next_header == 43 || next_header == 60) {
            if (offset + 2 > packet_end) break;
            const auto length = (static_cast<std::size_t>(packet.payload[offset + 1]) + 1) * 8;
            next_header = packet.payload[offset];
            if (length > packet_end - offset) break;
            offset += length;
        } else if (next_header == 44) {
            if (offset + 8 > packet_end) break;
            const auto fragment = read_be16(packet.payload + offset + 2);
            if ((fragment & 0xfff9U) != 0) {
                report(ErrorCode::UnsupportedFeature, packet.input_offset,
                       "fragmented IPv6 TLV payload is not reassembled");
                return;
            }
            next_header = packet.payload[offset];
            offset += 8;
        } else if (next_header == 51) {
            if (offset + 2 > packet_end) break;
            const auto length = (static_cast<std::size_t>(packet.payload[offset + 1]) + 2) * 4;
            next_header = packet.payload[offset];
            if (length > packet_end - offset) break;
            offset += length;
        } else {
            report(ErrorCode::UnsupportedFeature, packet.input_offset,
                   "unsupported IPv6 next-header in TLV payload");
            return;
        }
    }
    if (next_header != 17 || offset + 8 > packet_end) {
        report(ErrorCode::MalformedInput, packet.input_offset,
               "invalid IPv6 extension-header or UDP layout");
        return;
    }

    const auto source_port = read_be16(packet.payload + offset);
    const auto destination_port = read_be16(packet.payload + offset + 2);
    const auto udp_length = static_cast<std::size_t>(read_be16(packet.payload + offset + 4));
    if (udp_length < 8 || udp_length != packet_end - offset) {
        report(ErrorCode::MalformedInput, packet.input_offset,
               "UDP length does not match IPv6 payload");
        return;
    }
    if (destination_port != 123) {
        report(ErrorCode::UnsupportedFeature, packet.input_offset,
               "uncompressed IPv6 UDP payload is not NTP");
        return;
    }
    const auto* ntp = packet.payload + offset + 8;
    if (udp_length - 8 < 48) {
        report(ErrorCode::MalformedInput, packet.input_offset,
               "truncated NTP payload in IPv6 TLV packet");
        return;
    }

    TransportNtpClock clock;
    std::copy_n(packet.payload + 8, 16, clock.source_address.begin());
    std::copy_n(packet.payload + 24, 16, clock.destination_address.begin());
    clock.source_port = source_port;
    clock.destination_port = destination_port;
    clock.leap_indicator = static_cast<std::uint8_t>(ntp[0] >> 6U);
    clock.version = static_cast<std::uint8_t>((ntp[0] >> 3U) & 0x07U);
    clock.mode = static_cast<std::uint8_t>(ntp[0] & 0x07U);
    clock.stratum = ntp[1];
    clock.poll = static_cast<std::int8_t>(ntp[2]);
    clock.precision = static_cast<std::int8_t>(ntp[3]);
    clock.root_delay = read_be32(ntp + 4);
    clock.root_dispersion = read_be32(ntp + 8);
    clock.reference_identification = read_be32(ntp + 12);
    clock.reference_timestamp = read_be64_local(ntp + 16);
    clock.origin_timestamp = read_be64_local(ntp + 24);
    clock.receive_timestamp = read_be64_local(ntp + 32);
    clock.transmit_timestamp = read_be64_local(ntp + 40);
    clock.transmit_time = Timestamp{expand_ntp_microseconds(clock.transmit_timestamp), 1000000};
    clock.input_offset = packet.input_offset;
    latest_ntp_ = clock.transmit_timestamp;
    on_ntp_(std::move(clock));
}

bool TlvTransportParser::observe_compressed_flow(
    const TlvPacketView& packet, const std::uint32_t context_id,
    const std::uint8_t sequence_number) {
    if (packet.size < 45) {
        report(ErrorCode::MalformedInput, packet.input_offset,
               "truncated compressed IPv6/UDP context header");
        return false;
    }
    const auto* header = packet.payload + 3;
    if ((header[0] >> 4U) != 6) {
        report(ErrorCode::MalformedInput, packet.input_offset,
               "compressed-IP mode 0x60 does not contain an IPv6 header");
        return false;
    }
    IpDataFlow flow;
    flow.context_id = context_id;
    flow.sequence_number = sequence_number;
    flow.next_header = header[4];
    std::copy_n(header + 6, 16, flow.source_address.begin());
    std::copy_n(header + 22, 16, flow.destination_address.begin());
    flow.source_port = read_be16(header + 38);
    flow.destination_port = read_be16(header + 40);
    flow.input_offset = packet.input_offset;
    if (flow.next_header != 17) {
        report(ErrorCode::UnsupportedFeature, packet.input_offset,
               "compressed-IP mode 0x60 flow is not UDP");
    }
    if (flow.source_port != 50000 || flow.destination_port != 51216) {
        report(ErrorCode::UnsupportedFeature, packet.input_offset,
               "compressed-IP UDP ports differ from observed receiver flow 50000 -> 51216");
    }
    const auto found = flows_.find(context_id);
    if (found == flows_.end() || found->second.ip_version != flow.ip_version ||
        found->second.source_address != flow.source_address ||
        found->second.destination_address != flow.destination_address ||
        found->second.next_header != flow.next_header ||
        found->second.source_port != flow.source_port ||
        found->second.destination_port != flow.destination_port) {
        flows_[context_id] = flow;
        on_flow_(std::move(flow));
    }
    return true;
}

void TlvTransportParser::consume_tlv_si(const TlvPacketView& packet) {
    section_buffer_.insert(section_buffer_.end(), packet.payload, packet.payload + packet.size);
    section_origins_.insert(section_origins_.end(), packet.size, packet.input_offset);
    process_sections();
}

void TlvTransportParser::consume_null(const TlvPacketView&) {
    ++null_packet_count_;
}

void TlvTransportParser::process_sections() {
    const auto find_complete_boundary = [this](const std::size_t begin)
        -> std::optional<std::size_t> {
        for (std::size_t offset = begin; offset + 3 <= section_buffer_.size(); ++offset) {
            if ((section_buffer_[offset + 1] & 0x80U) == 0) continue;
            const auto length = static_cast<std::size_t>(
                (static_cast<std::uint16_t>(section_buffer_[offset + 1] & 0x0fU) << 8U) |
                section_buffer_[offset + 2]);
            if (length < 9 || length > 1021) continue;
            const auto total = 3 + length;
            if (total > section_buffer_.size() - offset) continue;
            if (section_buffer_[offset + 6] > section_buffer_[offset + 7]) continue;
            if (crc32_mpeg(section_buffer_.data() + offset, total) == 0) return offset;
        }
        return std::nullopt;
    };
    const auto discard_prefix = [this](const std::size_t size, const char* message) {
        report(ErrorCode::MalformedInput, section_origins_.front(), message);
        section_buffer_.erase(
            section_buffer_.begin(),
            section_buffer_.begin() + static_cast<std::ptrdiff_t>(size));
        section_origins_.erase(
            section_origins_.begin(),
            section_origins_.begin() + static_cast<std::ptrdiff_t>(size));
    };

    while (section_buffer_.size() >= 3) {
        const auto section_length = static_cast<std::size_t>(
            (static_cast<std::uint16_t>(section_buffer_[1] & 0x0fU) << 8U) |
            section_buffer_[2]);
        if ((section_buffer_[1] & 0x80U) == 0 || section_length < 9 ||
            section_length > 1021) {
            const auto boundary = find_complete_boundary(1);
            discard_prefix(boundary.value_or(1),
                           "discarded bytes while searching for a valid TLV-SI section");
            continue;
        }
        const auto total = 3 + section_length;
        if (section_buffer_.size() < total) {
            const auto boundary = find_complete_boundary(1);
            if (!boundary) return;
            discard_prefix(*boundary,
                           "discarded incomplete TLV-SI data before a valid section");
            continue;
        }
        if (section_buffer_[6] > section_buffer_[7] ||
            crc32_mpeg(section_buffer_.data(), total) != 0) {
            const auto boundary = find_complete_boundary(1);
            if (boundary) {
                discard_prefix(*boundary,
                               "discarded invalid TLV-SI data before a valid section");
            } else {
                discard_prefix(total, "TLV-SI section CRC or numbering is invalid");
            }
            continue;
        }
        std::vector<std::uint8_t> section(section_buffer_.begin(),
                                          section_buffer_.begin() + static_cast<std::ptrdiff_t>(total));
        const auto input_offset = section_origins_.front();
        section_buffer_.erase(section_buffer_.begin(),
                              section_buffer_.begin() + static_cast<std::ptrdiff_t>(total));
        section_origins_.erase(section_origins_.begin(),
                               section_origins_.begin() + static_cast<std::ptrdiff_t>(total));
        process_section(section, input_offset);
    }
}

void TlvTransportParser::process_section(const std::vector<std::uint8_t>& section,
                                         const std::uint64_t input_offset) {
    if (section.size() < 8 || (section[1] & 0x80U) == 0) {
        report(ErrorCode::MalformedInput, input_offset,
               "TLV-SI table does not use extended section syntax");
        return;
    }
    RawSignallingTable raw;
    raw.table_id = section[0];
    raw.table_id_extension = read_be16(section.data() + 3);
    raw.version = static_cast<std::uint8_t>((section[5] >> 1U) & 0x1fU);
    raw.current_next = (section[5] & 1U) != 0;
    raw.section_number = section[6];
    raw.last_section_number = section[7];
    raw.data = section;
    raw.input_offset = input_offset;
    on_raw_table_(std::move(raw));

    if (section[0] == 0x40 || section[0] == 0x41) {
        if (auto parsed = parse_nit(section, input_offset)) accept_nit(std::move(*parsed));
    } else if (section[0] == 0xfe && read_be16(section.data() + 3) == 0) {
        if (auto parsed = parse_address_map(section, input_offset)) {
            accept_address_map(std::move(*parsed));
        }
    }
}

std::optional<std::vector<TlvDescriptor>> TlvTransportParser::parse_descriptors(
    const std::vector<std::uint8_t>& section, const std::size_t begin,
    const std::size_t length, const std::uint8_t table_id, const DescriptorScope scope,
    const std::optional<std::uint16_t> tlv_stream_id,
    const std::optional<std::uint16_t> original_network_id,
    const std::uint64_t input_offset) {
    if (begin > section.size() || length > section.size() - begin) return std::nullopt;
    std::vector<TlvDescriptor> result;
    std::size_t cursor = begin;
    const auto end = begin + length;
    while (cursor < end) {
        if (end - cursor < 2) return std::nullopt;
        const auto tag = section[cursor];
        const auto descriptor_length = static_cast<std::size_t>(section[cursor + 1]);
        if (descriptor_length > end - cursor - 2) return std::nullopt;
        TlvDescriptor descriptor;
        descriptor.tag = tag;
        descriptor.section_offset = static_cast<std::uint16_t>(cursor);
        descriptor.payload.assign(section.begin() + static_cast<std::ptrdiff_t>(cursor + 2),
                                  section.begin() + static_cast<std::ptrdiff_t>(cursor + 2 + descriptor_length));
        if (!known_tlv_descriptor(tag)) {
            UnknownDescriptor unknown;
            unknown.table_id = table_id;
            unknown.tag = tag;
            unknown.scope = scope;
            unknown.tlv_stream_id = tlv_stream_id;
            unknown.original_network_id = original_network_id;
            unknown.section_offset = descriptor.section_offset;
            unknown.payload = descriptor.payload;
            unknown.input_offset = input_offset;
            on_unknown_descriptor_(std::move(unknown));
        }
        result.push_back(std::move(descriptor));
        cursor += 2 + descriptor_length;
    }
    return result;
}

std::optional<TlvTransportParser::NitSection> TlvTransportParser::parse_nit(
    const std::vector<std::uint8_t>& section, const std::uint64_t input_offset) {
    if (section.size() < 16 || section[6] > section[7]) {
        report(ErrorCode::MalformedInput, input_offset, "invalid TLV-NIT section header");
        return std::nullopt;
    }
    NitSection result;
    result.table_id = section[0];
    result.network_id = read_be16(section.data() + 3);
    result.version = static_cast<std::uint8_t>((section[5] >> 1U) & 0x1fU);
    result.current_next = (section[5] & 1U) != 0;
    result.section_number = section[6];
    result.last_section_number = section[7];
    result.input_offset = input_offset;
    std::size_t cursor = 8;
    const auto payload_end = section.size() - 4;
    if (cursor + 2 > payload_end) return std::nullopt;
    const auto network_length = static_cast<std::size_t>(read_be16(section.data() + cursor) & 0x0fffU);
    cursor += 2;
    auto network = parse_descriptors(section, cursor, network_length, result.table_id,
                                     DescriptorScope::Network, std::nullopt,
                                     std::nullopt, input_offset);
    if (!network) {
        report(ErrorCode::MalformedInput, input_offset, "invalid TLV-NIT network descriptor loop");
        return std::nullopt;
    }
    result.network_descriptors = std::move(*network);
    cursor += network_length;
    if (cursor + 2 > payload_end) return std::nullopt;
    const auto stream_loop_length = static_cast<std::size_t>(read_be16(section.data() + cursor) & 0x0fffU);
    cursor += 2;
    if (stream_loop_length != payload_end - cursor) {
        report(ErrorCode::MalformedInput, input_offset, "invalid TLV-NIT stream loop length");
        return std::nullopt;
    }
    const auto stream_end = cursor + stream_loop_length;
    while (cursor < stream_end) {
        if (stream_end - cursor < 6) return std::nullopt;
        TlvNetworkStream stream;
        stream.tlv_stream_id = read_be16(section.data() + cursor);
        stream.original_network_id = read_be16(section.data() + cursor + 2);
        const auto length = static_cast<std::size_t>(read_be16(section.data() + cursor + 4) & 0x0fffU);
        cursor += 6;
        auto descriptors = parse_descriptors(
            section, cursor, length, result.table_id, DescriptorScope::TlvStream,
            stream.tlv_stream_id, stream.original_network_id, input_offset);
        if (!descriptors) {
            report(ErrorCode::MalformedInput, input_offset, "invalid TLV-NIT stream descriptor loop");
            return std::nullopt;
        }
        stream.descriptors = std::move(*descriptors);
        result.streams.push_back(std::move(stream));
        cursor += length;
    }
    return result;
}

std::optional<TlvTransportParser::AddressMapSection>
TlvTransportParser::parse_address_map(const std::vector<std::uint8_t>& section,
                                      const std::uint64_t input_offset) {
    if (section.size() < 14 || section[6] > section[7]) {
        report(ErrorCode::MalformedInput, input_offset, "invalid AMT section header");
        return std::nullopt;
    }
    AddressMapSection result;
    result.table_id_extension = read_be16(section.data() + 3);
    result.version = static_cast<std::uint8_t>((section[5] >> 1U) & 0x1fU);
    result.current_next = (section[5] & 1U) != 0;
    result.section_number = section[6];
    result.last_section_number = section[7];
    result.input_offset = input_offset;
    const auto service_count = static_cast<std::size_t>(read_be16(section.data() + 8) >> 6U);
    std::size_t cursor = 10;
    const auto payload_end = section.size() - 4;
    for (std::size_t index = 0; index < service_count; ++index) {
        if (payload_end - cursor < 4) return std::nullopt;
        AddressMapService service;
        service.service_id = read_be16(section.data() + cursor);
        const auto loop_header = read_be16(section.data() + cursor + 2);
        const bool ipv6 = (loop_header & 0x8000U) != 0;
        service.ip_version = ipv6 ? 6 : 4;
        const auto loop_length = static_cast<std::size_t>(loop_header & 0x03ffU);
        cursor += 4;
        if (loop_length > payload_end - cursor) return std::nullopt;
        const auto fixed_length = !ipv6 ? std::size_t{10} : std::size_t{34};
        if (loop_length < fixed_length) return std::nullopt;
        if (!ipv6) {
            std::copy_n(section.data() + cursor, 4, service.source_address.begin());
            service.source_prefix_length = section[cursor + 4];
            std::copy_n(section.data() + cursor + 5, 4, service.destination_address.begin());
            service.destination_prefix_length = section[cursor + 9];
            if (service.source_prefix_length > 32 || service.destination_prefix_length > 32) {
                report(ErrorCode::MalformedInput, input_offset, "AMT IPv4 prefix length exceeds 32");
                return std::nullopt;
            }
        } else {
            std::copy_n(section.data() + cursor, 16, service.source_address.begin());
            service.source_prefix_length = section[cursor + 16];
            std::copy_n(section.data() + cursor + 17, 16, service.destination_address.begin());
            service.destination_prefix_length = section[cursor + 33];
            if (service.source_prefix_length > 128 || service.destination_prefix_length > 128) {
                report(ErrorCode::MalformedInput, input_offset, "AMT IPv6 prefix length exceeds 128");
                return std::nullopt;
            }
        }
        service.private_data.assign(
            section.begin() + static_cast<std::ptrdiff_t>(cursor + fixed_length),
            section.begin() + static_cast<std::ptrdiff_t>(cursor + loop_length));
        result.services.push_back(std::move(service));
        cursor += loop_length;
    }
    if (cursor != payload_end) {
        report(ErrorCode::MalformedInput, input_offset, "AMT service count does not consume section payload");
        return std::nullopt;
    }
    return result;
}

void TlvTransportParser::accept_nit(NitSection section) {
    for (auto it = nit_assemblies_.begin(); it != nit_assemblies_.end();) {
        if (std::get<0>(it->first) == section.table_id &&
            std::get<1>(it->first) == section.network_id &&
            (std::get<2>(it->first) != section.version ||
             std::get<3>(it->first) != section.current_next)) {
            it = nit_assemblies_.erase(it);
        } else {
            ++it;
        }
    }
    const TableKey key{section.table_id, section.network_id,
                       section.version, section.current_next};
    auto& assembly = nit_assemblies_[key];
    if (!assembly.sections.empty() && assembly.last_section_number != section.last_section_number) {
        assembly = {};
    }
    assembly.last_section_number = section.last_section_number;
    const auto found = assembly.sections.find(section.section_number);
    if (found != assembly.sections.end() &&
        found->second.network_descriptors == section.network_descriptors &&
        found->second.streams == section.streams) return;
    assembly.sections[section.section_number] = std::move(section);
    assembly.emitted = false;
    if (assembly.sections.size() != static_cast<std::size_t>(assembly.last_section_number) + 1) return;
    for (unsigned number = 0; number <= assembly.last_section_number; ++number) {
        if (assembly.sections.find(static_cast<std::uint8_t>(number)) == assembly.sections.end()) return;
    }
    TlvNetworkInformation snapshot;
    snapshot.table_id = std::get<0>(key);
    snapshot.network_id = std::get<1>(key);
    snapshot.version = std::get<2>(key);
    snapshot.current_next = std::get<3>(key);
    snapshot.last_section_number = assembly.last_section_number;
    snapshot.input_offset = assembly.sections.begin()->second.input_offset;
    for (const auto& [number, value] : assembly.sections) {
        (void)number;
        snapshot.network_descriptors.insert(snapshot.network_descriptors.end(),
            value.network_descriptors.begin(), value.network_descriptors.end());
        snapshot.streams.insert(snapshot.streams.end(), value.streams.begin(), value.streams.end());
    }
    on_nit_(std::move(snapshot));
    assembly.emitted = true;
}

void TlvTransportParser::accept_address_map(AddressMapSection section) {
    for (auto it = address_map_assemblies_.begin(); it != address_map_assemblies_.end();) {
        if (std::get<0>(it->first) == 0xfe &&
            std::get<1>(it->first) == section.table_id_extension &&
            (std::get<2>(it->first) != section.version ||
             std::get<3>(it->first) != section.current_next)) {
            it = address_map_assemblies_.erase(it);
        } else {
            ++it;
        }
    }
    const TableKey key{0xfe, section.table_id_extension,
                       section.version, section.current_next};
    auto& assembly = address_map_assemblies_[key];
    if (!assembly.sections.empty() && assembly.last_section_number != section.last_section_number) {
        assembly = {};
    }
    assembly.last_section_number = section.last_section_number;
    const auto found = assembly.sections.find(section.section_number);
    if (found != assembly.sections.end() && found->second.services == section.services) return;
    assembly.sections[section.section_number] = std::move(section);
    assembly.emitted = false;
    if (assembly.sections.size() != static_cast<std::size_t>(assembly.last_section_number) + 1) return;
    for (unsigned number = 0; number <= assembly.last_section_number; ++number) {
        if (assembly.sections.find(static_cast<std::uint8_t>(number)) == assembly.sections.end()) return;
    }
    AddressMap snapshot;
    snapshot.table_id_extension = std::get<1>(key);
    snapshot.version = std::get<2>(key);
    snapshot.current_next = std::get<3>(key);
    snapshot.last_section_number = assembly.last_section_number;
    snapshot.input_offset = assembly.sections.begin()->second.input_offset;
    for (const auto& [number, value] : assembly.sections) {
        (void)number;
        snapshot.services.insert(snapshot.services.end(),
                                 value.services.begin(), value.services.end());
    }
    on_address_map_(std::move(snapshot));
    assembly.emitted = true;
}

} // namespace aribtlv::detail
